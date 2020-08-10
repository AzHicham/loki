pub mod transit_data;

pub mod init;

pub(super) mod timetables;

mod calendar;

pub mod queries;

pub mod time;

pub mod iters;


pub struct LaxatipsData {
    pub transit_data : transit_data::TransitData,
    pub model :  transit_model::Model,
}

impl<'model> LaxatipsData {
    pub fn new(model :  transit_model::Model, 
        default_transfer_duration : time::PositiveDuration
    ) -> Self
    {
        let transit_data = transit_data::TransitData::new(&model, default_transfer_duration);
        Self {
            transit_data,
            model
        }
    }
}