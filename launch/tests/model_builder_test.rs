// Copyright (C) 2017 Hove and/or its affiliates.
//
// This program is free software: you can redistribute it and/or modify it
// under the terms of the GNU Affero General Public License as published by the
// Free Software Foundation, version 3.

// This program is distributed in the hope that it will be useful, but WITHOUT
// ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS
// FOR A PARTICULAR PURPOSE. See the GNU Affero General Public License for more
// details.

// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>

mod utils;

#[cfg(test)]
mod test {
    use super::utils::model_builder::ModelBuilder;
    #[test]
    fn simple_model_creation() {
        let model = ModelBuilder::default()
            .vj("toto", |vj_builder| {
                vj_builder.st("A", "10:00:00").st("B", "11:00:00");
            })
            .vj("tata", |vj_builder| {
                vj_builder.st("C", "10:00:00").st("D", "11:00:00");
            })
            .build();

        assert_eq!(
            model.get_corresponding_from_idx(model.vehicle_journeys.get_idx("toto").unwrap()),
            ["A", "B"]
                .iter()
                .map(|s| model.stop_points.get_idx(s).unwrap())
                .collect()
        );
        assert_eq!(
            model.get_corresponding_from_idx(model.vehicle_journeys.get_idx("tata").unwrap()),
            ["C", "D"]
                .iter()
                .map(|s| model.stop_points.get_idx(s).unwrap())
                .collect()
        );
        let default_calendar = model.calendars.get("default_service").unwrap();
        let dates = [loki::transit_model::objects::Date::from_ymd(2020, 1, 1)]
            .iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(default_calendar.dates, dates);
    }

    #[test]
    fn same_sp_model_creation() {
        let model = ModelBuilder::default()
            .vj("toto", |vj| {
                vj.st("A", "10:00:00").st("B", "11:00:00");
            })
            .vj("tata", |vj| {
                vj.st("A", "10:00:00").st("D", "11:00:00");
            })
            .build();

        assert_eq!(
            model.get_corresponding_from_idx(model.vehicle_journeys.get_idx("toto").unwrap()),
            ["A", "B"]
                .iter()
                .map(|s| model.stop_points.get_idx(s).unwrap())
                .collect()
        );
        assert_eq!(
            model.get_corresponding_from_idx(model.stop_points.get_idx("A").unwrap()),
            ["toto", "tata"]
                .iter()
                .map(|s| model.vehicle_journeys.get_idx(s).unwrap())
                .collect()
        );

        assert_eq!(model.stop_points.len(), 3);
        assert_eq!(model.stop_areas.len(), 3);
    }

    #[test]
    fn model_creation_with_lines() {
        let model = ModelBuilder::default()
            .route("1", |r| {
                r.name = "bob".into();
            })
            .vj("toto", |vj_builder| {
                vj_builder
                    .route("1")
                    .st("A", "10:00:00")
                    .st("B", "11:00:00");
            })
            .vj("tata", |vj_builder| {
                vj_builder
                    .route("2")
                    .st("C", "10:00:00")
                    .st("D", "11:00:00");
            })
            .vj("tutu", |vj_builder| {
                vj_builder.st("C", "10:00:00").st("E", "11:00:00");
            })
            .build();

        assert_eq!(
            model.get_corresponding_from_idx(model.vehicle_journeys.get_idx("toto").unwrap()),
            ["A", "B"]
                .iter()
                .map(|s| model.stop_points.get_idx(s).unwrap())
                .collect()
        );
        assert_eq!(
            model.get_corresponding_from_idx(model.vehicle_journeys.get_idx("tata").unwrap()),
            ["C", "D"]
                .iter()
                .map(|s| model.stop_points.get_idx(s).unwrap())
                .collect()
        );
        // there should be only 3 routes, the route '1', '2' and the default one for 'tutu'
        assert_eq!(model.routes.len(), 3);
        assert_eq!(
            model.get_corresponding_from_idx(model.routes.get_idx("1").unwrap()),
            ["toto"]
                .iter()
                .map(|s| model.vehicle_journeys.get_idx(s).unwrap())
                .collect()
        );
        assert_eq!(
            model.get_corresponding_from_idx(model.routes.get_idx("2").unwrap()),
            ["tata"]
                .iter()
                .map(|s| model.vehicle_journeys.get_idx(s).unwrap())
                .collect()
        );
        assert_eq!(model.routes.get("1").unwrap().name, "bob");
        assert_eq!(
            model.get_corresponding_from_idx(model.routes.get_idx("default_route").unwrap()),
            ["tutu"]
                .iter()
                .map(|s| model.vehicle_journeys.get_idx(s).unwrap())
                .collect()
        );
    }
}
