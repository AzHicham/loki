use crate::{
    chaos::sql_types::{
        ChannelType as ChannelTypeSQL, DisruptionStatus, ImpactStatus, PtObjectType, SeverityEffect,
    },
    info,
    server_config::ChaosParams,
};
use anyhow::{bail, format_err, Error};
use diesel::{
    pg::types::sql_types::Array,
    prelude::*,
    sql_types::{Bit, Bool, Date, Int4, Nullable, Text, Time, Timestamp, Uuid},
};
use launch::loki::models::real_time_disruption::{
    Impacted, Informed, LineId, LineSectionDisruption, NetworkId, RailSectionDisruption, RouteId,
    StopAreaId, StopPointId,
};
use launch::loki::{
    chrono::{NaiveDate, NaiveTime},
    models::real_time_disruption::{
        ApplicationPattern, ChannelType, DateTimePeriod, Disruption, DisruptionProperty, Effect,
        Impact, Message, Severity, Tag, TimeSlot,
    },
    NaiveDateTime,
};
use std::collections::{
    hash_map::Entry::{Occupied, Vacant},
    HashMap, HashSet,
};
use uuid::Uuid as Uid;

pub fn chaos_disruption_from_database(
    config: &ChaosParams,
    publication_period: (NaiveDate, NaiveDate),
    contributors: &[String],
) -> Result<(), Error> {
    let connection = PgConnection::establish(&config.chaos_database)?;

    info!("Querying chaos database");
    let res = diesel::sql_query(include_str!("query.sql"))
        .bind::<Date, _>(publication_period.1)
        .bind::<Date, _>(publication_period.0)
        .bind::<Date, _>(publication_period.1)
        .bind::<Array<Text>, _>(contributors)
        .bind::<Int4, _>(config.chaos_batch_size as i32)
        .bind::<Int4, _>(config.chaos_batch_size as i32)
        .load::<ChaosDisruption>(&connection);

    let mut disruption_maker = DisruptionMaker::default();

    if let Err(ref err) = res {
        println!("{}", err);
    }

    info!("Converting database rows into Disruption");
    for row in res.unwrap() {
        print!(".");
        if let Err(ref err) = disruption_maker.read_disruption(&row) {
            println!("\n{}", err);
        }
    }
    info!("\nDisruptions ready to be applied");

    Ok(())
}

#[derive(Default)]
struct DisruptionMaker {
    pub(crate) disruptions: Vec<Disruption>,
    pub(crate) disruptions_set: HashMap<Uid, usize>,
    pub(crate) impacts_set: HashMap<Uid, usize>,
    pub(crate) tags_set: HashSet<Uid>,
    pub(crate) properties_set: HashSet<(String, String, String)>,

    pub(crate) impact_object_set: ImpactMaker,
}

impl DisruptionMaker {
    pub fn read_disruption(&mut self, row: &ChaosDisruption) -> Result<(), Error> {
        let find_disruption = self.disruptions_set.entry(row.disruption_id);
        let disruption = match find_disruption {
            Vacant(entry) => {
                let publication_period = if let (Some(start), Some(end)) = (
                    row.disruption_start_publication_date,
                    row.disruption_end_publication_date,
                ) {
                    DateTimePeriod::new(start, end)?
                } else {
                    return Err(format_err!(""));
                };
                let disruption = Disruption {
                    id: row.disruption_id.to_string(),
                    reference: row.disruption_reference.clone(),
                    contributor: Some(row.contributor.clone()),
                    publication_period,
                    created_at: None,
                    updated_at: None,
                    cause: Default::default(),
                    tags: vec![],
                    properties: vec![],
                    impacts: vec![],
                };
                // clear all set related to disruption
                self.impacts_set.clear();
                self.tags_set.clear();
                self.properties_set.clear();
                self.impact_object_set.clear();
                self.disruptions.push(disruption);
                let idx: usize = self.disruptions.len() - 1;
                entry.insert(idx);
                self.disruptions.last_mut().unwrap()
            }
            Occupied(entry) => self.disruptions.get_mut(*entry.get()).unwrap(),
        };
        DisruptionMaker::update_tags(&mut self.tags_set, row, disruption)?;
        DisruptionMaker::update_properties(&mut self.properties_set, row, disruption)?;
        DisruptionMaker::update_impacts(
            &mut self.impact_object_set,
            &mut self.impacts_set,
            row,
            disruption,
        )?;
        Ok(())
    }

    fn update_tags(
        tags_set: &mut HashSet<Uid>,
        row: &ChaosDisruption,
        disruption: &mut Disruption,
    ) -> Result<(), Error> {
        if let Some(tag_id) = row.tag_id {
            let name = if let Some(name) = &row.tag_name {
                name
            } else {
                bail!("Tag has no name");
            };
            if !tags_set.contains(&tag_id) {
                disruption.tags.push(Tag {
                    id: tag_id.to_string(),
                    name: name.clone(),
                });
                tags_set.insert(tag_id);
            }
        }
        Ok(())
    }

    fn update_properties(
        properties_set: &mut HashSet<(String, String, String)>,
        row: &ChaosDisruption,
        disruption: &mut Disruption,
    ) -> Result<(), Error> {
        // type_ is here like an Uuid
        if let Some(type_) = &row.property_type {
            let key = if let Some(key) = &row.property_key {
                key
            } else {
                bail!("Property has no key");
            };
            let value = if let Some(value) = &row.property_value {
                value
            } else {
                bail!("Property has no value");
            };
            let tuple = (type_.clone(), key.clone(), value.clone());
            if !properties_set.contains(&tuple) {
                disruption.properties.push(DisruptionProperty {
                    key: key.clone(),
                    type_: type_.clone(),
                    value: value.clone(),
                });
                properties_set.insert(tuple);
            }
        }
        Ok(())
    }

    fn update_impacts(
        impact_object_set: &mut ImpactMaker,
        impacts_set: &mut HashMap<Uid, usize>,
        row: &ChaosDisruption,
        disruption: &mut Disruption,
    ) -> Result<(), Error> {
        let impact = if let Some(idx) = impacts_set.get(&row.impact_id) {
            // Impact already in disruption
            disruption.impacts.get_mut(*idx).unwrap()
        } else {
            // Or create a new impact We must then clear all sub-objects of impact
            impact_object_set.clear();

            let impact = Impact {
                id: row.impact_id.to_string(),
                created_at: None,
                updated_at: None,
                application_periods: vec![],
                application_patterns: vec![],
                severity: Severity {
                    id: row.severity_id.to_string(),
                    wording: Some(row.severity_wording.clone()),
                    color: row.severity_color.clone(),
                    priority: Some(row.severity_priority),
                    effect: row
                        .severity_effect
                        .as_ref()
                        .map_or(Effect::UnknownEffect, |e| {
                            DisruptionMaker::make_severity_effect(e)
                        }),
                    created_at: None,
                    updated_at: None,
                },
                messages: vec![],
                impacted_pt_objects: vec![],
                informed_pt_objects: vec![],
            };

            disruption.impacts.push(impact);
            let idx: usize = disruption.impacts.len() - 1;
            impacts_set.insert(row.impact_id, idx);
            disruption.impacts.last_mut().unwrap()
        };

        ImpactMaker::update_application_period(
            &mut impact_object_set.application_periods_set,
            row,
            impact,
        )?;
        ImpactMaker::update_messages(&mut impact_object_set.messages_set, row, impact)?;
        ImpactMaker::update_application_pattern(
            &mut impact_object_set.application_pattern_set,
            row,
            impact,
        )?;
        ImpactMaker::update_pt_objects(&mut impact_object_set.pt_objects_set, row, impact)?;

        Ok(())
    }

    fn make_severity_effect(effect: &SeverityEffect) -> Effect {
        match effect {
            SeverityEffect::NoService => Effect::NoService,
            SeverityEffect::OtherEffect => Effect::OtherEffect,
            SeverityEffect::ModifiedService => Effect::ModifiedService,
            SeverityEffect::AdditionalService => Effect::AdditionalService,
            SeverityEffect::StopMoved => Effect::StopMoved,
            SeverityEffect::SignificantDelays => Effect::SignificantDelays,
            SeverityEffect::ReducedService => Effect::ReducedService,
            SeverityEffect::UnknownEffect => Effect::UnknownEffect,
            SeverityEffect::Detour => Effect::Detour,
        }
    }
}

#[derive(Default)]
struct ImpactMaker {
    pub(crate) application_periods_set: HashSet<Uid>,
    pub(crate) application_pattern_set: HashSet<Uid>,
    pub(crate) messages_set: HashSet<Uid>,
    pub(crate) pt_objects_set: HashSet<Uid>,
}

impl ImpactMaker {
    fn clear(&mut self) {
        self.application_periods_set.clear();
        self.application_pattern_set.clear();
        self.messages_set.clear();
        self.pt_objects_set.clear();
    }

    fn update_application_period(
        application_periods_set: &mut HashSet<Uid>,
        row: &ChaosDisruption,
        impact: &mut Impact,
    ) -> Result<(), Error> {
        if !application_periods_set.contains(&row.application_id) {
            let start_date = if let Some(start_date) = row.application_start_date {
                start_date
            } else {
                bail!("ApplicationPeriod has no start_date");
            };
            let end_date = if let Some(end_date) = row.application_end_date {
                end_date
            } else {
                bail!("ApplicationPeriod has no start_date");
            };
            impact
                .application_periods
                .push(DateTimePeriod::new(start_date, end_date)?);
        }
        Ok(())
    }

    fn update_messages(
        messages_set: &mut HashSet<Uid>,
        row: &ChaosDisruption,
        impact: &mut Impact,
    ) -> Result<(), Error> {
        if let Some(message_id) = row.message_id {
            let text = if let Some(text) = &row.message_text {
                text
            } else {
                bail!("Message has no text");
            };
            let channel_name = if let Some(channel_name) = &row.channel_name {
                channel_name
            } else {
                bail!("Message has no channel_name");
            };
            if !messages_set.contains(&message_id) {
                impact.messages.push(Message {
                    text: text.clone(),
                    channel_id: row.channel_id.map(|channel_id| channel_id.to_string()),
                    channel_name: channel_name.clone(),
                    channel_content_type: row.channel_content_type.clone(),
                    channel_types: row
                        .channel_type
                        .iter()
                        .filter_map(|c| c.as_ref().map(ImpactMaker::make_channel_type))
                        .collect(),
                })
            }
        }
        Ok(())
    }

    fn make_channel_type(channel_type: &ChannelTypeSQL) -> ChannelType {
        match channel_type {
            ChannelTypeSQL::Title => ChannelType::Title,
            ChannelTypeSQL::Beacon => ChannelType::Beacon,
            ChannelTypeSQL::Twitter => ChannelType::Twitter,
            ChannelTypeSQL::Notification => ChannelType::Notification,
            ChannelTypeSQL::Sms => ChannelType::Sms,
            ChannelTypeSQL::Facebook => ChannelType::Facebook,
            ChannelTypeSQL::Email => ChannelType::Email,
            ChannelTypeSQL::Mobile => ChannelType::Mobile,
            ChannelTypeSQL::Web => ChannelType::Web,
        }
    }

    fn update_application_pattern(
        application_pattern_set: &mut HashSet<Uid>,
        row: &ChaosDisruption,
        impact: &mut Impact,
    ) -> Result<(), Error> {
        if let Some(pattern_id) = row.pattern_id {
            if !application_pattern_set.contains(&pattern_id) {
                let begin_date = if let Some(begin_date) = row.pattern_start_date {
                    begin_date
                } else {
                    bail!("Pattern has no start_date");
                };
                let end_date = if let Some(end_date) = row.pattern_end_date {
                    end_date
                } else {
                    bail!("Pattern has no end_date");
                };

                // time_slot_begin && time_slot_end have always the same size
                // thanks to the sql query
                let time_slots = row
                    .time_slot_begin
                    .iter()
                    .filter_map(|begin| *begin)
                    .zip(row.time_slot_end.iter().filter_map(|end| *end))
                    .map(|(begin, end)| TimeSlot { begin, end })
                    .collect();
                impact.application_patterns.push(ApplicationPattern {
                    begin_date,
                    end_date,
                    time_slots,
                })
            }
        }
        Ok(())
    }

    fn update_pt_objects(
        pt_object_set: &mut HashSet<Uid>,
        row: &ChaosDisruption,
        impact: &mut Impact,
    ) -> Result<(), Error> {
        let id = if let Some(id) = &row.ptobject_uri {
            id.clone()
        } else {
            bail!("PtObject has no uri");
        };
        let impacted = &mut impact.impacted_pt_objects;
        let informed = &mut impact.informed_pt_objects;

        match (&row.ptobject_type, impact.severity.effect) {
            (PtObjectType::Network, Effect::NoService) => {
                impacted.push(Impacted::NetworkDeleted(NetworkId { id }));
            }
            (PtObjectType::Network, _) => {
                informed.push(Informed::Network(NetworkId { id }));
            }

            (PtObjectType::Route, Effect::NoService) => {
                impacted.push(Impacted::RouteDeleted(RouteId { id }));
            }
            (PtObjectType::Route, _) => {
                informed.push(Informed::Route(RouteId { id }));
            }

            (PtObjectType::Line, Effect::NoService) => {
                impacted.push(Impacted::LineDeleted(LineId { id }));
            }
            (PtObjectType::Line, _) => {
                informed.push(Informed::Line(LineId { id }));
            }

            (PtObjectType::StopPoint, Effect::NoService) => {
                impacted.push(Impacted::StopPointDeleted(StopPointId { id }));
            }
            (PtObjectType::StopPoint, _) => {
                informed.push(Informed::StopPoint(StopPointId { id }));
            }

            (PtObjectType::StopArea, Effect::NoService) => {
                impacted.push(Impacted::StopAreaDeleted(StopAreaId { id }));
            }
            (PtObjectType::StopArea, _) => {
                informed.push(Informed::StopArea(StopAreaId { id }));
            }

            (PtObjectType::LineSection, _) => {
                // check if we need to create a new line section or just push a new route into it
                let found_line_section: Option<&mut LineSectionDisruption> = impacted
                    .iter_mut()
                    .filter_map(|pt| match pt {
                        Impacted::LineSection(line_section) => Some(line_section),
                        _ => None,
                    })
                    .find(|line_section| line_section.id == id);

                match found_line_section {
                    Some(line_section) => {
                        // we found line_section so we push a new route
                        // if not already in line_section.routes[]
                        if let Some(route_id) = &row.ls_route_uri {
                            let found_route = line_section
                                .routes
                                .iter()
                                .find(|route| route.id == *route_id);
                            if let None = found_route {
                                line_section.routes.push(RouteId {
                                    id: route_id.clone(),
                                });
                            }
                        }
                    }
                    None => {
                        let line_id = if let Some(line_id) = &row.ls_line_uri {
                            line_id.clone()
                        } else {
                            bail!("PtObject has type line_section but the field line_id is empty");
                        };
                        let start = if let Some(start) = &row.ls_start_uri {
                            start.clone()
                        } else {
                            bail!("PtObject has type line_section but the field start is empty");
                        };
                        let end = if let Some(end) = &row.ls_end_uri {
                            end.clone()
                        } else {
                            bail!("PtObject has type line_section but the field end is empty");
                        };
                        let routes = if let Some(route_id) = &row.ls_route_uri {
                            vec![RouteId {
                                id: route_id.clone(),
                            }]
                        } else {
                            vec![]
                        };

                        let line_section = LineSectionDisruption {
                            id,
                            line: LineId { id: line_id },
                            start: StopAreaId { id: start },
                            end: StopAreaId { id: end },
                            routes,
                        };
                        impacted.push(Impacted::LineSection(line_section));
                    }
                }
            }
            (PtObjectType::RailSection, _) => {
                // check if we need to create a new rail section or just push a new route into it
                let found_rail_section: Option<&mut RailSectionDisruption> = impacted
                    .iter_mut()
                    .filter_map(|pt| match pt {
                        Impacted::RailSection(rail_section) => Some(rail_section),
                        _ => None,
                    })
                    .find(|rail_section| rail_section.id == id);

                match found_rail_section {
                    Some(rail_section) => {
                        // we found rail_section so we push a new route
                        // if not already in rail_section.routes[]
                        if let Some(route_id) = &row.rs_route_uri {
                            let found_route = rail_section
                                .routes
                                .iter()
                                .find(|route| route.id == *route_id);
                            if let None = found_route {
                                rail_section.routes.push(RouteId {
                                    id: route_id.clone(),
                                });
                            }
                        }
                    }
                    None => {
                        let line_id = if let Some(line_id) = &row.rs_line_uri {
                            line_id.clone()
                        } else {
                            bail!("PtObject has type rail_section but the field line_id is empty");
                        };
                        let start = if let Some(start) = &row.rs_start_uri {
                            start.clone()
                        } else {
                            bail!("PtObject has type rail_section but the field start is empty");
                        };
                        let end = if let Some(end) = &row.rs_end_uri {
                            end.clone()
                        } else {
                            bail!("PtObject has type rail_section but the field end is empty");
                        };

                        let line_section = RailSectionDisruption {
                            id,
                            line: LineId { id: line_id },
                            start: StopAreaId { id: start },
                            end: StopAreaId { id: end },
                            routes: vec![],
                        };
                        impacted.push(Impacted::RailSection(line_section));
                    }
                }
            }
        };

        Ok(())
    }
}

// Remove ChaosDisruption when PR https://github.com/diesel-rs/diesel/pull/2254 is merged
// and use model_v2
#[derive(Queryable, QueryableByName, Debug)]
pub struct ChaosDisruption {
    // Disruptions field
    #[sql_type = "Uuid"]
    pub disruption_id: Uid,
    #[sql_type = "Nullable<Text>"]
    pub disruption_reference: Option<String>,
    #[sql_type = "crate::chaos::sql_types::disruption_status"]
    pub disruption_status: DisruptionStatus,
    #[sql_type = "Nullable<Timestamp>"]
    pub disruption_start_publication_date: Option<NaiveDateTime>,
    #[sql_type = "Nullable<Timestamp>"]
    pub disruption_end_publication_date: Option<NaiveDateTime>,
    #[sql_type = "Timestamp"]
    pub disruption_created_at: NaiveDateTime,
    #[sql_type = "Nullable<Timestamp>"]
    pub disruption_updated_at: Option<NaiveDateTime>,
    #[sql_type = "Text"]
    pub contributor: String,
    // Cause fields
    #[sql_type = "Uuid"]
    pub cause_id: Uid,
    #[sql_type = "Text"]
    pub cause_wording: String,
    #[sql_type = "diesel::sql_types::Bool"]
    pub cause_visible: bool,
    #[sql_type = "Timestamp"]
    pub cause_created_at: NaiveDateTime,
    #[sql_type = "Nullable<Timestamp>"]
    pub cause_updated_at: Option<NaiveDateTime>,
    // Category fields
    #[sql_type = "Nullable<Text>"]
    pub category_name: Option<String>,
    #[sql_type = "Nullable<Uuid>"]
    pub category_id: Option<Uid>,
    #[sql_type = "Nullable<Timestamp>"]
    pub category_created_at: Option<NaiveDateTime>,
    #[sql_type = "Nullable<Timestamp>"]
    pub category_updated_at: Option<NaiveDateTime>,
    // Tag fields
    #[sql_type = "Nullable<Uuid>"]
    pub tag_id: Option<Uid>,
    #[sql_type = "Nullable<Text>"]
    pub tag_name: Option<String>,
    #[sql_type = "Nullable<Bool>"]
    pub tag_is_visible: Option<bool>,
    #[sql_type = "Nullable<Timestamp>"]
    pub tag_created_at: Option<NaiveDateTime>,
    #[sql_type = "Nullable<Timestamp>"]
    pub tag_updated_at: Option<NaiveDateTime>,
    // Impact fields
    #[sql_type = "Uuid"]
    pub impact_id: Uid,
    #[sql_type = "crate::chaos::sql_types::impact_status"]
    pub impact_status: ImpactStatus,
    #[sql_type = "Nullable<Uuid>"]
    pub impact_disruption_id: Option<Uid>,
    #[sql_type = "Timestamp"]
    pub impact_created_at: NaiveDateTime,
    #[sql_type = "Nullable<Timestamp>"]
    pub impact_updated_at: Option<NaiveDateTime>,
    // Application period fields
    #[sql_type = "Uuid"]
    pub application_id: Uid,
    #[sql_type = "Nullable<Timestamp>"]
    pub application_start_date: Option<NaiveDateTime>,
    #[sql_type = "Nullable<Timestamp>"]
    pub application_end_date: Option<NaiveDateTime>,
    // Severity fields
    #[sql_type = "Uuid"]
    pub severity_id: Uid,
    #[sql_type = "Text"]
    pub severity_wording: String,
    #[sql_type = "Nullable<Text>"]
    pub severity_color: Option<String>,
    #[sql_type = "Bool"]
    pub severity_is_visible: bool,
    #[sql_type = "Int4"]
    pub severity_priority: i32,
    #[sql_type = "Nullable<crate::chaos::sql_types::severity_effect>"]
    pub severity_effect: Option<SeverityEffect>,
    #[sql_type = "Timestamp"]
    pub severity_created_at: NaiveDateTime,
    #[sql_type = "Nullable<Timestamp>"]
    pub severity_updated_at: Option<NaiveDateTime>,
    // Ptobject fields
    #[sql_type = "Uuid"]
    pub ptobject_id: Uid,
    #[sql_type = "crate::chaos::sql_types::pt_object_type"]
    pub ptobject_type: PtObjectType,
    #[sql_type = "Nullable<Text>"]
    pub ptobject_uri: Option<String>,
    #[sql_type = "Timestamp"]
    pub ptobject_created_at: NaiveDateTime,
    #[sql_type = "Nullable<Timestamp>"]
    pub ptobject_updated_at: Option<NaiveDateTime>,
    // Ptobject line_section fields
    #[sql_type = "Nullable<Text>"]
    pub ls_line_uri: Option<String>,
    #[sql_type = "Nullable<Timestamp>"]
    pub ls_line_created_at: Option<NaiveDateTime>,
    #[sql_type = "Nullable<Timestamp>"]
    pub ls_line_updated_at: Option<NaiveDateTime>,
    #[sql_type = "Nullable<Text>"]
    pub ls_start_uri: Option<String>,
    #[sql_type = "Nullable<Timestamp>"]
    pub ls_start_created_at: Option<NaiveDateTime>,
    #[sql_type = "Nullable<Timestamp>"]
    pub ls_start_updated_at: Option<NaiveDateTime>,
    #[sql_type = "Nullable<Text>"]
    pub ls_end_uri: Option<String>,
    #[sql_type = "Nullable<Timestamp>"]
    pub ls_end_created_at: Option<NaiveDateTime>,
    #[sql_type = "Nullable<Timestamp>"]
    pub ls_end_updated_at: Option<NaiveDateTime>,
    #[sql_type = "Nullable<Uuid>"]
    pub ls_route_id: Option<Uid>,
    #[sql_type = "Nullable<Text>"]
    pub ls_route_uri: Option<String>,
    #[sql_type = "Nullable<Timestamp>"]
    pub ls_route_created_at: Option<NaiveDateTime>,
    #[sql_type = "Nullable<Timestamp>"]
    pub ls_route_updated_at: Option<NaiveDateTime>,
    // Ptobject rail_section fields
    #[sql_type = "Nullable<Text>"]
    pub rs_line_uri: Option<String>,
    #[sql_type = "Nullable<Timestamp>"]
    pub rs_line_created_at: Option<NaiveDateTime>,
    #[sql_type = "Nullable<Timestamp>"]
    pub rs_line_updated_at: Option<NaiveDateTime>,
    #[sql_type = "Nullable<Text>"]
    pub rs_start_uri: Option<String>,
    #[sql_type = "Nullable<Timestamp>"]
    pub rs_start_created_at: Option<NaiveDateTime>,
    #[sql_type = "Nullable<Timestamp>"]
    pub rs_start_updated_at: Option<NaiveDateTime>,
    #[sql_type = "Nullable<Text>"]
    pub rs_end_uri: Option<String>,
    #[sql_type = "Nullable<Timestamp>"]
    pub rs_end_created_at: Option<NaiveDateTime>,
    #[sql_type = "Nullable<Timestamp>"]
    pub rs_end_updated_at: Option<NaiveDateTime>,
    #[sql_type = "Nullable<Uuid>"]
    pub rs_route_id: Option<Uid>,
    #[sql_type = "Nullable<Text>"]
    pub rs_route_uri: Option<String>,
    #[sql_type = "Nullable<Timestamp>"]
    pub rs_route_created_at: Option<NaiveDateTime>,
    #[sql_type = "Nullable<Timestamp>"]
    pub rs_route_updated_at: Option<NaiveDateTime>,
    // Message fields
    #[sql_type = "Nullable<Uuid>"]
    pub message_id: Option<Uid>,
    #[sql_type = "Nullable<Text>"]
    pub message_text: Option<String>,
    #[sql_type = "Nullable<Timestamp>"]
    pub message_created_at: Option<NaiveDateTime>,
    #[sql_type = "Nullable<Timestamp>"]
    pub message_updated_at: Option<NaiveDateTime>,
    // Channel fields
    #[sql_type = "Nullable<Uuid>"]
    pub channel_id: Option<Uid>,
    #[sql_type = "Nullable<Text>"]
    pub channel_name: Option<String>,
    #[sql_type = "Nullable<Text>"]
    pub channel_content_type: Option<String>,
    #[sql_type = "Nullable<Int4>"]
    pub channel_max_size: Option<i32>,
    #[sql_type = "Nullable<Timestamp>"]
    pub channel_created_at: Option<NaiveDateTime>,
    #[sql_type = "Nullable<Timestamp>"]
    pub channel_updated_at: Option<NaiveDateTime>,
    // #[sql_type = "Nullable<Uuid>"]
    //  pub channel_type_id: Option<Uid>,
    #[sql_type = "Array<Nullable<crate::chaos::sql_types::channel_type_enum>>"]
    pub channel_type: Vec<Option<ChannelTypeSQL>>,
    //  Property & Associate property fields
    #[sql_type = "Nullable<Text>"]
    pub property_value: Option<String>,
    #[sql_type = "Nullable<Text>"]
    pub property_key: Option<String>,
    #[sql_type = "Nullable<Text>"]
    pub property_type: Option<String>,
    // Pattern & TimeSlot fields
    #[sql_type = "Nullable<Date>"]
    pub pattern_start_date: Option<NaiveDate>,
    #[sql_type = "Nullable<Date>"]
    pub pattern_end_date: Option<NaiveDate>,
    #[sql_type = "Nullable<Bit>"]
    pub pattern_weekly_pattern: Option<Vec<u8>>,
    #[sql_type = "Nullable<Uuid>"]
    pub pattern_id: Option<Uid>,
    #[sql_type = "Array<Nullable<Time>>"]
    pub time_slot_begin: Vec<Option<NaiveTime>>,
    #[sql_type = "Array<Nullable<Time>>"]
    pub time_slot_end: Vec<Option<NaiveTime>>,
    // #[sql_type = "Nullable<Uuid>"]
    //pub time_slot_id: Option<Uid>,
}
