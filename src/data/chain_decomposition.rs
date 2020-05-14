

pub struct ChainDecomposition<Id, Value> {
    nb_of_positions : usize,
    chains : Vec<Chain<Id, Value>>
}

impl<Id, Value> ChainDecomposition<Id, Value> 
where Value : Ord + Clone 
{
    pub fn new(nb_of_positions : usize) -> Self
    {
        Self {
            nb_of_positions,
            chains : Vec::new()
        }
    }

    pub fn insert(& mut self, values : & [Value], id : Id) {
        debug_assert!( values.len() == self.nb_of_positions );
        for chain in self.chains.iter_mut() {
            if chain.accept(values) {
                chain.insert(values, id);
                return;
            }
        }
        let mut new_chain = Chain::new(self.nb_of_positions);
        new_chain.insert(values, id);
        self.chains.push(new_chain);
    }
}

struct Chain<Id, Value> {
    // ids, ordered by increasing value in the first position
    ids : Vec<Id>,

    // values_by_position[position][id]
    // is the value of the vector identified by `by` 
    // at `position`
    // so for each `position`, `values_by_position[position]`
    // is a vector of size `ids.len()`
    values_by_position : Vec<Vec<Value>>  
}


impl<Id, Value> Chain<Id, Value>
where Value : Ord + Clone 
{

    fn new(nb_of_positions : usize) -> Self {
        assert!( nb_of_positions >= 1);
        Chain{
            ids : Vec::new(),
            values_by_position : vec![Vec::new(); nb_of_positions]
        }
    }

    fn nb_of_positions(&self) -> usize {
        self.values_by_position.len()
    }

    fn id_values<'a>(& 'a self, id_idx : usize) -> IdValuesIter<'a, Id, Value> {
        debug_assert!( id_idx < self.ids.len() );
        IdValuesIter {
            chain : & self,
            id_idx,
            position : 0
        }
    }

    //
    // test whether, for all id_values vector 
    // we have either 
    //    - values[pos] <= id_values[pos] for all pos
    //    - values[pos] >= id_values[pos] for all pos
    // if this happens, we say that the two vectors are "comparable"
    pub fn accept(& self, values :&[Value]) -> bool {
        use std::cmp::Ordering;
        debug_assert!( values.len() == self.nb_of_positions() );
        for id_idx in 0..self.ids.len() {
            let id_values = self.id_values(id_idx);
            let zip_iter = values.iter().zip(id_values);
            let mut first_not_equal_iter = zip_iter.skip_while(|(left, right)| **left == right.clone());
            let has_first_not_equal = first_not_equal_iter.next();
            if let Some(first_not_equal) = has_first_not_equal {
                let ordering = first_not_equal.0.cmp(&first_not_equal.1);
                assert!( ordering != Ordering::Equal);
                // let's see if there is a position where the ordering is not the same
                // as first_ordering
                let found = first_not_equal_iter.find(|(left, right)| {
                    let cmp = left.cmp(&right);
                    cmp != ordering && cmp != Ordering::Equal
                });
                if found.is_some() {
                    return false;
                }
                // if found.is_none(), it means that 
                // all elements are ordered the same, so the two vectors are comparable

            }
            // if has_first_not_equal == None
            // then values == id_values
            // the two vector are comparable
        }
        true
    }

    pub fn insert(& mut self,  values :&[Value],  id : Id)
    {
        debug_assert!(self.accept(values));
        //let's find where to insert our new times vector
        let first_value = values[0].clone();
        debug_assert!(self.is_sorted());
        let search_insert_idx = self.values_by_position[0].binary_search(&first_value);
        let insert_idx = match search_insert_idx {
            Result::Ok(idx) => idx,
            Result::Err(idx) => idx 
        };
        for pos in 0..self.nb_of_positions() {
            self.values_by_position[pos].insert(insert_idx, values[pos].clone());
        }
        self.ids.insert(insert_idx, id);
    }

    fn is_sorted(&self) -> bool {
        for pos in 0..self.nb_of_positions() {
            let pos_values = &self.values_by_position[pos];
            let pos_sorted = (0..self.ids.len() - 1).all(|i| pos_values[i] <= pos_values[i + 1]);
            if ! pos_sorted {
                return false;
            }
        }
        true
    }
}

struct IdValuesIter<'a, Id, Value> {
    chain : & 'a Chain<Id, Value>,
    id_idx : usize,
    position : usize,
}

impl<'a, Id, Value>  Iterator for IdValuesIter<'a, Id, Value>
where Value : Clone
{
    type Item = Value;

    fn next(&mut self) -> Option<Self::Item> {
        if self.position >= self.chain.values_by_position.len() {
            None
        }
        else {
            let result = self.chain.values_by_position[self.position][self.id_idx].clone();
            self.position = self.position + 1;
            Some(result)
            
        }
    }
}
