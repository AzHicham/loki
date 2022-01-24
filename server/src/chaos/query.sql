SELECT
d.id as disruption_id, d.reference as disruption_reference, d.note as disruption_note,
d.status as disruption_status,
d.start_publication_date  AT TIME ZONE 'UTC' as disruption_start_publication_date,
d.end_publication_date  AT TIME ZONE 'UTC' as disruption_end_publication_date,
d.created_at  AT TIME ZONE 'UTC' as disruption_created_at,
d.updated_at  AT TIME ZONE 'UTC' as disruption_updated_at,
co.contributor_code as contributor,

c.id as cause_id, c.wording as cause_wording,
c.is_visible as cause_visible,
c.created_at  AT TIME ZONE 'UTC' as cause_created_at,
c.updated_at  AT TIME ZONE 'UTC' as cause_updated_at,

cat.name as category_name, cat.id as category_id,
cat.created_at  AT TIME ZONE 'UTC' as category_created_at,
cat.updated_at  AT TIME ZONE 'UTC' as category_updated_at,

t.id as tag_id, t.name as tag_name, t.is_visible as tag_is_visible,
t.created_at  AT TIME ZONE 'UTC' as tag_created_at,
t.updated_at  AT TIME ZONE 'UTC' as tag_updated_at,

i.id as impact_id, i.status as impact_status, i.disruption_id as impact_disruption_id,
i.created_at  AT TIME ZONE 'UTC' as impact_created_at,
i.updated_at  AT TIME ZONE 'UTC' as impact_updated_at,

a.id as application_id,
a.start_date  AT TIME ZONE 'UTC' as application_start_date,
a.end_date  AT TIME ZONE 'UTC' as application_end_date,

s.id as severity_id, s.wording as severity_wording, s.color as severity_color,
s.is_visible as severity_is_visible, s.priority as severity_priority,
s.effect as severity_effect,
s.created_at  AT TIME ZONE 'UTC' as severity_created_at,
s.updated_at  AT TIME ZONE 'UTC' as severity_updated_at,

p.id as ptobject_id, p.type as ptobject_type, p.uri as ptobject_uri,
p.created_at  AT TIME ZONE 'UTC' as ptobject_created_at,
p.updated_at  AT TIME ZONE 'UTC' as ptobject_updated_at,

ls_line.uri as ls_line_uri,
ls_line.created_at  AT TIME ZONE 'UTC' as ls_line_created_at,
ls_line.updated_at  AT TIME ZONE 'UTC' as ls_line_updated_at,
ls_start.uri as ls_start_uri,
ls_start.created_at  AT TIME ZONE 'UTC' as ls_start_created_at,
ls_start.updated_at  AT TIME ZONE 'UTC' as ls_start_updated_at,
ls_end.uri as ls_end_uri,
ls_end.created_at  AT TIME ZONE 'UTC' as ls_end_created_at,
ls_end.updated_at  AT TIME ZONE 'UTC' as ls_end_updated_at,
ls_route.id AS ls_route_id,
ls_route.uri AS ls_route_uri,
ls_route.created_at  AT TIME ZONE 'UTC' as ls_route_created_at,
ls_route.updated_at  AT TIME ZONE 'UTC' as ls_route_updated_at,

rs_line.id as rs_line_id,
rs_line.uri as rs_line_uri,
rs_line.created_at  AT TIME ZONE 'UTC' as rs_line_created_at,
rs_line.updated_at  AT TIME ZONE 'UTC' as rs_line_updated_at,
rs_start.uri as rs_start_uri,
rs_start.created_at  AT TIME ZONE 'UTC' as rs_start_created_at,
rs_start.updated_at  AT TIME ZONE 'UTC' as rs_start_updated_at,
rs_end.uri as rs_end_uri,
rs_end.created_at  AT TIME ZONE 'UTC' as rs_end_created_at,
rs_end.updated_at  AT TIME ZONE 'UTC' as rs_end_updated_at,
rs_route.id AS rs_route_id,
rs_route.uri AS rs_route_uri,
rs_route.created_at  AT TIME ZONE 'UTC' as rs_route_created_at,
rs_route.updated_at  AT TIME ZONE 'UTC' as rs_route_updated_at,
rail_section.blocked_stop_areas as rs_blocked_sa,

m.id as message_id, m.text as message_text,
m.created_at  AT TIME ZONE 'UTC' as message_created_at,
m.updated_at  AT TIME ZONE 'UTC' as message_updated_at,

ch.id as channel_id, ch.name as channel_name,
ch.content_type as channel_content_type, ch.max_size as channel_max_size,
ch.created_at  AT TIME ZONE 'UTC' as channel_created_at,
ch.updated_at  AT TIME ZONE 'UTC' as channel_updated_at,
cht.id as channel_type_id, cht.name as channel_type,

adp.value as property_value, pr.key as property_key, pr.type as property_type,

pt.start_date as pattern_start_date,
pt.end_date as pattern_end_date,
pt.weekly_pattern as pattern_weekly_pattern,
pt.id as pattern_id,
ts.begin as time_slot_begin,
ts.end  as time_slot_end,
ts.id as time_slot_id

FROM disruption AS d
    JOIN contributor AS co ON d.contributor_id = co.id
    JOIN cause AS c ON (c.id = d.cause_id)
    LEFT JOIN category AS cat ON cat.id=c.category_id
    LEFT JOIN associate_disruption_tag ON associate_disruption_tag.disruption_id = d.id
    LEFT JOIN tag AS t ON associate_disruption_tag.tag_id = t.id
    JOIN impact AS i ON i.disruption_id = d.id
    JOIN application_periods AS a ON a.impact_id = i.id
    JOIN severity AS s ON s.id = i.severity_id
    JOIN associate_impact_pt_object ON associate_impact_pt_object.impact_id = i.id
    JOIN pt_object AS p ON associate_impact_pt_object.pt_object_id = p.id
    LEFT JOIN line_section ON p.id = line_section.object_id
    LEFT JOIN pt_object AS ls_line ON line_section.line_object_id = ls_line.id
    LEFT JOIN pt_object AS ls_start ON line_section.start_object_id = ls_start.id
    LEFT JOIN pt_object AS ls_end ON line_section.end_object_id = ls_end.id
    LEFT JOIN associate_line_section_route_object
    ON associate_line_section_route_object.line_section_id = line_section.id
    LEFT JOIN pt_object AS ls_route
    ON associate_line_section_route_object.route_object_id = ls_route.id
    LEFT JOIN rail_section ON p.id = rail_section.object_id
    LEFT JOIN pt_object AS rs_line ON rail_section.line_object_id = rs_line.id
    LEFT JOIN pt_object AS rs_start ON rail_section.start_object_id = rs_start.id
    LEFT JOIN pt_object AS rs_end ON rail_section.end_object_id = rs_end.id
    LEFT JOIN associate_rail_section_route_object
    ON associate_rail_section_route_object.rail_section_id = rail_section.id
    LEFT JOIN pt_object AS rs_route
    ON associate_rail_section_route_object.route_object_id = rs_route.id
    LEFT JOIN message AS m ON m.impact_id = i.id
    LEFT JOIN channel AS ch ON m.channel_id = ch.id
    LEFT JOIN channel_type as cht on ch.id = cht.channel_id
    LEFT JOIN associate_disruption_property adp ON adp.disruption_id = d.id
    LEFT JOIN property pr ON pr.id = adp.property_id
    LEFT JOIN pattern AS pt ON pt.impact_id = i.id
    LEFT JOIN time_slot AS ts ON ts.pattern_id = pt.id
WHERE (
    NOT (d.start_publication_date >= $1 OR d.end_publication_date <= $2)
   OR (d.start_publication_date <= $3 and d.end_publication_date IS NULL)
    )
  AND co.contributor_code = ANY($4)
  AND d.status = 'published'
  AND i.status = 'published'
ORDER BY d.id, c.id, t.id, i.id, m.id, ch.id, cht.id
LIMIT $5 OFFSET $6