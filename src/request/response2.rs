use crate::transit_data::{
    data::{Stop, Transfer, Trip, TransitData},
    ordered_timetable::Position,
    time::{SecondsSinceDatasetStart, PositiveDuration},
};
use transit_model::Model;

#[derive(Debug, Clone)]
pub struct VehicleLeg {
    pub trip: Trip,
    pub board_position: Position,
    pub debark_position: Position,
}

#[derive(Debug, Clone)]
pub struct Journey {
    departure_datetime : SecondsSinceDatasetStart,
    departure_fallback_duration : PositiveDuration,
    first_vehicle: VehicleLeg,
    connections: Vec<(Transfer, VehicleLeg)>,
    arrival_fallback_duration: PositiveDuration,
}
#[derive(Debug, Clone)]
pub enum VehicleLegIdx {
    First,
    Connection(usize)
}
#[derive(Debug, Clone)]
pub enum BadJourney {
    DebarkIsUpstreamBoard(VehicleLeg, VehicleLegIdx), 
    NoBoardTime(VehicleLeg, VehicleLegIdx),
    NoDebarkTime(VehicleLeg, VehicleLegIdx),
    BadTransferStartStop(VehicleLeg, Transfer, usize),
    BadTransferEndStop(Transfer, VehicleLeg, usize),
    BadTransferEndTime(Transfer, VehicleLeg, usize),
}

impl Journey {

    pub fn new(    
        departure_datetime : SecondsSinceDatasetStart,
        departure_fallback_duration : PositiveDuration,
        first_vehicle: VehicleLeg,
        connections: impl Iterator<Item = (Transfer, VehicleLeg)>,
        arrival_fallback_duration: PositiveDuration,
        transit_data : & TransitData,
    ) -> Result<Self, BadJourney> {
        let result = Self {
            departure_datetime,
            departure_fallback_duration,
            first_vehicle,
            arrival_fallback_duration,
            connections : connections.collect()
        };

        result.is_valid(transit_data)?;

        Ok(result)
    }

    fn is_valid(&self, transit_data : & TransitData) -> Result<(), BadJourney> {
        let (first_debark_stop, first_debark_time) =  {
            
            let board_position = &self.first_vehicle.board_position;
            let debark_position = &self.first_vehicle.debark_position;
            let trip = &self.first_vehicle.trip;
            let mission = transit_data.mission_of(trip);
            if transit_data.is_upstream_in_mission(
                debark_position,
                board_position, 
                &mission
            ) {
                return Err(BadJourney::DebarkIsUpstreamBoard(self.first_vehicle.clone(), VehicleLegIdx::First))
            } 
        
            if transit_data.board_time_of(trip, board_position).is_none() {
                return Err(BadJourney::NoBoardTime(self.first_vehicle.clone(), VehicleLegIdx::First))
            }
        
            let debark_time = transit_data.debark_time_of(trip, debark_position)
                .ok_or_else(|| {
                    BadJourney::NoDebarkTime(self.first_vehicle.clone(), VehicleLegIdx::First)
                })?;

            let debark_stop = transit_data.stop_at_position_in_mission(debark_position, &mission);
            (debark_stop, debark_time)
        };

        let mut prev_debark_stop = first_debark_stop;
        let mut prev_debark_time = first_debark_time;
        let mut prev_vehicle_leg = &self.first_vehicle;

        for (idx, (transfer, vehicle_leg)) in self.connections.iter().enumerate() {
            let transfer_start_stop = transit_data.transfer_start_stop(transfer);
            if prev_debark_stop != transfer_start_stop {
                return Err(BadJourney::BadTransferStartStop(prev_vehicle_leg.clone(), transfer.clone(), idx));
            }
            let (transfer_end_stop, transfer_duration) = transit_data.transfer(transfer);

            let board_position = &vehicle_leg.board_position;
            let debark_position = &vehicle_leg.debark_position;
            let trip = &vehicle_leg.trip;
            let mission = transit_data.mission_of(trip);
            if transit_data.is_upstream_in_mission(
                debark_position,
                board_position, 
                &mission
            ) {
                return Err(BadJourney::DebarkIsUpstreamBoard(vehicle_leg.clone(), VehicleLegIdx::Connection(idx)))
            } 

            let board_time = transit_data.board_time_of(trip, board_position)
                .ok_or_else(|| {
                    BadJourney::NoBoardTime(vehicle_leg.clone(), VehicleLegIdx::Connection(idx))
                })?;

        
            let debark_time = transit_data.debark_time_of(trip, debark_position)
                .ok_or_else(|| {
                    BadJourney::NoDebarkTime(vehicle_leg.clone(), VehicleLegIdx::Connection(idx))
                })?;

            let board_stop = transit_data.stop_at_position_in_mission(board_position, &mission);
            let debark_stop = transit_data.stop_at_position_in_mission(debark_position, &mission);

            if transfer_end_stop != board_stop {
                return Err(BadJourney::BadTransferEndStop(transfer.clone(), vehicle_leg.clone(), idx))
            }

            let end_transfer_time = prev_debark_time.clone() + transfer_duration;
            if end_transfer_time > board_time {
                return Err(BadJourney::BadTransferEndTime(transfer.clone(), vehicle_leg.clone(), idx))
            }

            prev_debark_time = debark_time;
            prev_debark_stop = debark_stop;
            prev_vehicle_leg = vehicle_leg;

        }

        Ok(())

    }

    pub fn arrival_time(&self, transit_data : & TransitData) -> SecondsSinceDatasetStart {
        let last_vehicle_leg = self.connections.last()
            .map(|(_, vehicle_leg)| vehicle_leg)
            .unwrap_or(&self.first_vehicle);
        let last_debark_time = transit_data
            .debark_time_of(&last_vehicle_leg.trip, &last_vehicle_leg.debark_position)
            .unwrap(); //unwrap is safe because of checks that happens during Self construction
        last_debark_time + self.arrival_fallback_duration
    }

    pub fn total_transfer_duration(&self, transit_data : & TransitData) -> PositiveDuration {
        let mut result = PositiveDuration::zero();
        for (transfer, _) in &self.connections {
            let (_, transfer_duration) = transit_data.transfer(transfer);
            result = result + transfer_duration;
        }
        result
    }

    pub fn nb_of_legs(&self) -> usize {
        self.connections.len() + 1
    }

    pub fn print(&self, 
        transit_data : & TransitData, 
        model : & Model, 
    ) -> Result<String, std::fmt::Error> {
        let mut result = String::new();
        self.write(transit_data, model, & mut result)?;
        Ok(result)
        
    }

    pub fn write< Writer : std::fmt::Write>(&self, 
            transit_data : & TransitData, 
            model : & Model, 
            writer : & mut Writer
    ) -> Result<(), std::fmt::Error> {
        writeln!(writer, "*** New journey ***")?;
        let arrival_time = self.arrival_time(transit_data);
        writeln!(writer, "Arrival : {}", transit_data.calendar.to_pretty_string(&arrival_time ))?;
        writeln!(writer, "Transfer duration : {}", self.total_transfer_duration(transit_data))?;
        writeln!(writer, "Nb of vehicles : {}", self.nb_of_legs())?;
        
        writeln!(writer, "Departure : {}", transit_data.calendar.to_pretty_string(&self.departure_datetime))?;

        self.write_vehicle_leg(&self.first_vehicle, transit_data, model, writer)?;
        for (transfer, vehicle_leg) in self.connections.iter() {
            self.write_vehicle_leg(vehicle_leg, transit_data, model, writer)?;
        }

        Ok(())
    }

    fn write_vehicle_leg< Writer : std::fmt::Write>(&self, 
        vehicle_leg : & VehicleLeg, 
        transit_data : & TransitData, 
        model : & Model,
        writer : & mut Writer
    ) -> Result<(), std::fmt::Error>
    {
        let trip = &vehicle_leg.trip;
        let vehicle_journey_idx = transit_data.vehicle_journey_idx(trip);
        let route_id = &model.vehicle_journeys[vehicle_journey_idx].route_id;
        let route_id = &model.routes.get(route_id).unwrap().id;

        let mission = transit_data.mission_of(trip);
        let from_stop = transit_data.stop_at_position_in_mission(&vehicle_leg.board_position, &mission);
        let to_stop = transit_data.stop_at_position_in_mission(&vehicle_leg.debark_position, &mission);
        let from_stop_idx = transit_data.stop_point_idx(&from_stop);
        let to_stop_idx = transit_data.stop_point_idx(&to_stop);
        let from_stop_id = &model.stop_points[from_stop_idx].id;
        let to_stop_id = &model.stop_points[to_stop_idx].id;

        let board_time = transit_data.board_time_of(trip, &vehicle_leg.board_position).unwrap();
        let debark_time = transit_data.debark_time_of(trip, &vehicle_leg.debark_position).unwrap();

        let from_datetime = transit_data.calendar.to_pretty_string(&board_time);
        let to_datetime = transit_data.calendar.to_pretty_string(&debark_time);
        writeln!(writer, "{} from {} at {} to {} at {} ", 
            route_id, 
            from_stop_id,
            from_datetime,
            to_stop_id,
            to_datetime
        )?;
        Ok(())
    }

    
}


// fn debark_stop(
//         vehicle_section : & VehicleSection, 
//         transit_data : & TransitData
//     ) -> Stop {
//     let debark_position = &vehicle_section.debark_position;
//     let trip = &vehicle_section.trip;
//     let mission = transit_data.mission_of(trip);
//     transit_data.stop_at_position_in_mission(debark_position, &mission)
// }

// fn board_stop(
//         vehicle_section : & VehicleSection, 
//         transit_data : & TransitData
//     ) -> Stop {
//     let board_position = &vehicle_section.board_position;
//     let trip = &vehicle_section.trip;
//     let mission = transit_data.mission_of(trip);
//     transit_data.stop_at_position_in_mission(board_position, &mission)
// }

// fn check_vehicle_section( 
//     vehicle_section : & VehicleSection, 
//     vehicle_section_idx : Option<usize>,
//     transit_data : & TransitData
// ) -> Result<(), BadJourney> {
//     let board_position = &vehicle_section.board_position;
//     let debark_position = &vehicle_section.debark_position;
//     let trip = &vehicle_section.trip;
//     let mission = transit_data.mission_of(trip);
//     if transit_data.is_upstream_in_mission(
//         debark_position,
//         board_position, 
//         &mission
//     ) {
//         return Err(BadJourney::DebarkIsUpstreamBoard(vehicle_section.clone(), vehicle_section_idx))
//     } 

//     if transit_data.board_time_of(trip, board_position).is_none() {
//         return Err(BadJourney::NoBoardTime(vehicle_section.clone(), vehicle_section_idx))
//     }

//     if transit_data.debark_time_of(trip, debark_position).is_none() {
//         return Err(BadJourney::NoDebarkTime(vehicle_section.clone(), vehicle_section_idx))
//     }

//     Ok(())
// }