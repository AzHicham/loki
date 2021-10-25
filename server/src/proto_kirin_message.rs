// Copyright  (C) 2020, Kisio Digital and/or its affiliates. All rights reserved.
//
// This file is part of Navitia,
// the software to build cool stuff with public transport.
//
// Hope you'll enjoy and contribute to this project,
// powered by Kisio Digital (www.kisio.com).
// Help us simplify mobility and open public transport:
// a non ending quest to the responsive locomotion way of traveling!
//
// This contribution is a part of the research and development work of the
// IVA Project which aims to enhance traveler information and is carried out
// under the leadership of the Technological Research Institute SystemX,
// with the partnership and support of the transport organization authority
// Ile-De-France Mobilités (IDFM), SNCF, and public funds
// under the scope of the French Program "Investissements d’Avenir".
//
// LICENCE: This program is free software; you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.
//
// Stay tuned using
// twitter @navitia
// channel `#navitia` on riot https://riot.im/app/#/room/#navitia:matrix.org
// https://groups.google.com/d/forum/navitia
// www.navitia.io

// Which field should I trust for deciding whether :
//  - this is a NEW vehicle (not in ntfs)
//  - this is a modification of an EXISTING vehicle (in ntfs, or a previously received new vehicle)
//  - this is a vehicle to be deleted.
// I can obtain contradictory informations from
//  - self.effect
//  - self.trip.schedule_relationship
//  - self.stop_time_updates[].schedule_relationship
//  - self.stop_time_updates[].departure.schedule_relationship
//  - self.stop_time_updates[].departure.stop_time_event_status
//  - self.stop_time_updates[].arrival.schedule_relationship
//  - self.stop_time_updates[].arrival.stop_time_event_status
pub struct TripUpdate {
    trip: TripDescriptor,
    // always present, right ?
    vehicle: Option<VehicleDescriptor>,

    // I guess (??) this is either  :
    //  - empty if this is a vehicle to be deleted
    //  - contains all stop_times if this is a NEW vehicle. In this case :
    //        - all stop_time_updates[].departure/arrival.schedule_relationship == ADDED ? Some of them may be == SCHEDULED ?
    //        - what about stop_time_updates[].departure/arrival.stop_time_event_status ?
    //  - contains all stop_times if this is a modification of an existing vehicle. In this case :
    //        - all stop_time_updates[].departure/arrival.schedule_relationship == SCHEDULED ? ADDED ?
    //        - what about stop_time_updates[].departure/arrival.stop_time_event_status ?
    stop_time_updates: Vec<StopTimeUpdate>,

    // never present ?
    timestamp: Option<u64>,

    // kirin extensions
    trip_message: Option<String>, // always here ?
    headsign: Option<String>,     // always here ?
    effect: Option<AlertEffect>,  // always here ?
}

// all values can happen ?
pub enum AlertEffect {
    NoService,
    ReducedService,
    SignificantDelay,
    Detour,
    AdditionnalService,
    ModifiedService,
    OtherEffect,
    UnknownEffect,
    StopMoved,
}

pub struct TripDescriptor {
    // always here, right ?
    // == vehicle_journey_id if this concerns a vehicle_journey found in the ntfs ?
    trip_id: String,

    // can be empty ?
    // What if :
    //  - trip_id == a vehicle_journey_id found in ntfs
    //  - route_id is present, but is != route(vehicle_journey_id) in the ntfs ?
    route_id: Option<String>,

    // always present, right ?
    // useless for added vehicle, right ?
    start_time: Option<String>,
    // always present, but useless because of past-midnight, right ?
    start_date: Option<String>,

    schedule_relationship: Option<TripDescriptorScheduleRelationship>,

    // kirin extensions
    // always present ?
    // same question as for route_id, what if :
    //  - trip_id == a vehicle_journey_id found in ntfs
    //  - these fields are present, but their values != value in ntfs ?
    contributor: Option<String>,
    company_id: Option<String>,
}

// all values can happen ?
enum TripDescriptorScheduleRelationship {
    Canceled,
    Added,
    Scheduled,

    // these two below never happen in Kirin, right ?
    Unscheduled,
    Replacement,
}

pub struct VehicleDescriptor {
    // kirin extension
    // always present ?
    // what if :
    //  - this concerns a vehicle_journey that exists in the NTFS
    //  - the physical_mode_id is different that the one in the NTFS ?
    physical_mode_id: Option<String>,

    // never present, right ?
    id: Option<String>,
    // never present, right ?
    label: Option<String>,
    // never present, right ?
    licence_plate: Option<String>,
}

pub struct StopTimeUpdate {
    // useless ?
    stop_sequence: Option<u32>,
    // always_present, right ?
    // in which case can it be a stop_id not present in NTFS ?
    stop_id: Option<String>,

    // always present ?
    arrival: Option<StopTimeEvent>,
    // always present ?
    departure: Option<StopTimeEvent>,

    // never present, right ?
    schedule_relationship: Option<StopTimeUpdateScheduleRelationship>,

    // what to do with that ?
    stoptime_message: Option<String>,
}

pub enum StopTimeUpdateScheduleRelationship {
    Scheduled,
    Skipped,
    Added,
    // never happens ?
    NoData,
}

pub struct StopTimeEvent {
    // alway present, but useless
    delay: Option<i32>,
    // always present, right ?
    // is a unix_timestamp (nb of seconds since Jan 1st 1970)
    // this will fail after 2038, right ? https://en.wikipedia.org/wiki/Year_2038_problem
    time: Option<i64>,

    // never present ?
    uncertainty: Option<i32>,

    // kirin extensions

    // always present, right ?
    stop_time_event_relationship: Option<StopTimeUpdateScheduleRelationship>,

    // this will always be consistent with self.stop_time_event_relationship ?
    // for exemple, can I have  :
    //  - stop_time_event_relationship == Added
    //  - and stop_time_event_status == Deleted ?
    stop_time_event_status: Option<StopTimeEventStatus>,
}

enum StopTimeEventStatus {
    // all these values can happen, right ?
    Scheduled,
    Deleted,
    Added,
    DeletedForDetour,
    AddedForDetour,
    // never happens, right ?
    NoData,
}
