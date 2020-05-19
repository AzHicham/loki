
use super::data::{
    TransitData,
    Stop,
    StopIdx,
    StopPattern,
    StopPatternIdx,
    Position,
};

impl Stop {

    // Returns 
    // - None if this Stop does not appears in `stop_pattern_idx`
    // - Some(position) otherwise, where `position` is the position of this Stop in the StopPattern
    fn get_position_in_stop_pattern(&self, stop_pattern_idx: & StopPatternIdx) -> Option<Position> {
        self.position_in_stop_patterns.iter()
            .find(|&(candidate_stop_pattern_idx, _)| {
                candidate_stop_pattern_idx == stop_pattern_idx
            })
            .map(|&(_, position)| position)
    }
}

impl TransitData {

    pub fn is_upstream(&self,
            upstream_idx : & StopIdx, 
            downstream_idx : & StopIdx,
            stop_pattern_idx : & StopPatternIdx 
    ) -> bool {
        let upstream = &self.stops[upstream_idx.idx];
        let dowstream = &self.stops[downstream_idx.idx];

        format!("The stop {:?} is expected to belongs to the stop_pattern {:?}", *upstream_idx, *stop_pattern_idx);

        let upstream_position = upstream
            .get_position_in_stop_pattern(stop_pattern_idx)
            .unwrap_or_else( || panic!(format!("The stop {:?} is expected to belongs to the stop_pattern {:?}", 
                                                    *upstream_idx, 
                                                    *stop_pattern_idx))
                            );

        let downstream_position = dowstream
            .get_position_in_stop_pattern(stop_pattern_idx)
            .unwrap_or_else( || panic!(format!("The stop {:?} is expected to belongs to the stop_pattern {:?}", 
                                                    *upstream_idx, 
                                                    *stop_pattern_idx))
                            );
        upstream_position.idx < downstream_position.idx

    }

    pub fn next_on_stop_pattern(&self, 
        stop_idx : & StopIdx,
        stop_pattern_idx : & StopPatternIdx,
    ) -> Option<StopIdx> {
        let stop = &self.stops[stop_idx.idx];
        let position = stop.get_position_in_stop_pattern(stop_pattern_idx)
            .unwrap_or_else(|| panic!(format!("The stop {:?} is expected to belongs to the stop_pattern {:?}", 
                                                *stop_idx, 
                                                *stop_pattern_idx))
                            );
        let stop_pattern = &self.stop_patterns[stop_pattern_idx.idx];
        debug_assert!(position.idx < stop_pattern.stops.len() );
        stop_pattern.stops.get(position.idx + 1).copied()
    }

}