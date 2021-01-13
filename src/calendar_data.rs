pub mod transit_data;

pub mod init;

pub(super) mod timetables;

pub mod days_patterns;

pub mod queries;


pub mod iters;

pub use crate::time::PositiveDuration;
pub use transit_data::{TransitData, Idx, StopPoint, VehicleJourney, TransitModelTransfer};


pub struct LaxatipsData {
    pub transit_data : transit_data::TransitData,
    pub model :  transit_model::Model,
}

impl<'model> LaxatipsData {
    pub fn new(model :  transit_model::Model, 
        default_transfer_duration : PositiveDuration
    ) -> Self
    {
        let transit_data = transit_data::TransitData::new(&model, default_transfer_duration);
        Self {
            transit_data,
            model
        }
    }
}