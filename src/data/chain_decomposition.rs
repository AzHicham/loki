use std::cmp::Ordering;
use super::data::Position;


pub struct ChainDecomposition<ItemData, Value> {
    nb_of_positions : usize,
    chains : Vec<Chain<ItemData, Value>>
}

struct Chain<ItemData, Value> {
    // items data, ordered by increasing value in the first position
    items_data : Vec<ItemData>,

    // values_by_position[position][id]
    // is the value of the item identified by `id` 
    // at `position`
    // so for each `position`, `values_by_position[position]`
    // is a vector of size `items_data.len()`
    values_by_position : Vec<Vec<Value>>  
}


pub struct ChainIdx {
    idx : usize  
}

pub struct ItemIdx {
    chain_idx : usize, // idx of the chain in its ChainDecomposition
    item_idx : usize,  // idx of the item in its Chain
}





impl<ItemData, Value> ChainDecomposition<ItemData, Value> 
where Value : Ord + Clone 
{
    pub fn new(nb_of_positions : usize) -> Self
    {
        Self {
            nb_of_positions,
            chains : Vec::new()
        }
    }

    pub fn nb_of_items(&self) -> usize {
        self.chains.iter().map(|chain| chain.nb_of_items()).sum()
    }

    pub fn insert(& mut self, values : & [Value], item_data : ItemData) {
        debug_assert!( values.len() == self.nb_of_positions );
        for chain in self.chains.iter_mut() {
            if chain.accept(values) {
                chain.insert(values, item_data);
                return;
            }
        }
        let mut new_chain = Chain::new(self.nb_of_positions);
        new_chain.insert(values, item_data);
        self.chains.push(new_chain);
    }

    pub fn get_value(&self, item_idx : & ItemIdx, position : & Position) -> &Value {
        self.chains[item_idx.chain_idx].value_at(item_idx.item_idx, position.idx)
    }

    pub fn get_data(&self, item_idx : & ItemIdx) -> & ItemData {
        & self.chains[item_idx.chain_idx].items_data[item_idx.item_idx]
    }

    pub fn get_items_greater_or_equal(&self, value : Value, position :  Position) -> impl Iterator<Item=ItemIdx> + '_{ 
        debug_assert!(position.idx < self.nb_of_positions);
        self.chains.iter().enumerate()
        .filter_map( move |(chain_idx, chain)| {
            chain.get_idx_of_first_item_greater_or_equal(&value, position.idx)
                    .map(|item_idx|{
                        ItemIdx {
                            chain_idx,
                            item_idx
                        }
                    })
        })
    }
}



impl<ItemData, Value> Chain<ItemData, Value>
where Value : Ord + Clone 
{

    fn new(nb_of_positions : usize) -> Self {
        assert!( nb_of_positions >= 1);
        Chain{
            items_data : Vec::new(),
            values_by_position : vec![Vec::new(); nb_of_positions]
        }
    }

    fn nb_of_positions(&self) -> usize {
        self.values_by_position.len()
    }

    fn nb_of_items(&self) -> usize {
        self.items_data.len()
    }

    fn value_at(&self, item_idx : usize, pos_idx : usize) -> & Value {
        &self.values_by_position[pos_idx][item_idx]
    }

    fn item_values<'a>(& 'a self, item_idx : usize) -> ItemValuesIter<'a, ItemData, Value> {
        debug_assert!( item_idx < self.items_data.len() );
        ItemValuesIter {
            chain : & self,
            item_idx,
            position : 0
        }
    }

    // If we denote item_values the vector present at `item_idx`, then returns
    //    - Some(Equal) if values[pos] == item_values[pos] for all pos
    //    - Some(Lower) if values[pos] <= item_values[pos] for all pos
    //    - Some(Upper) if values[pos] >= item_values[pos] for all pos
    //    - None otherwise (the two vector are not comparable)
    fn partial_cmp(&self, item_idx : usize, values : & [Value]) -> Option<Ordering> {
        debug_assert!( values.len() == self.nb_of_positions() );
        let item_values = self.item_values(item_idx);
        let zip_iter = values.iter().zip(item_values);
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
                return None;
            }
            // if found.is_none(), it means that 
            // all elements are ordered the same, so the two vectors are comparable
            return Some(ordering);
        }
        // if has_first_not_equal == None
        // then values == item_values
        // the two vector are equal
        return Some(Ordering::Equal);
        
    }


    fn accept(& self, values :&[Value]) -> bool {
        debug_assert!( values.len() == self.nb_of_positions() );
        for item_idx in 0..self.nb_of_items() {
            if self.partial_cmp(item_idx, values).is_none() {
                return false;
            }
        }
        true
    }

    fn insert(& mut self,  values :&[Value],  item_data : ItemData)
    {
        debug_assert!(self.accept(values));
        let nb_of_items = self.nb_of_items();
        //TODO : maybe start testing from the end ?
        let insert_idx = (0..nb_of_items).find(|&idx| {
            let partial_cmp = self.partial_cmp(idx, values); 
            partial_cmp ==  Some(Ordering::Equal)
            || partial_cmp == Some(Ordering::Greater)
        })
        .map(|idx| std::cmp::max(idx-1, 0) )
        .unwrap_or(nb_of_items);

        for pos in 0..self.nb_of_positions() {
            self.values_by_position[pos].insert(insert_idx, values[pos].clone());
        }
        self.items_data.insert(insert_idx, item_data);
    }

    fn get_idx_of_first_item_greater_or_equal(&self, value : & Value, position : usize) -> Option<usize> {
        debug_assert!(self.is_sorted());
        // TODO : do a binary_search instead of a line search ?
        let idx = self.values_by_position[position].iter().enumerate().find(|&(_, idx_val)| {
            value >= idx_val
        })
        .map(|(idx, idx_val)| {
            if *idx_val == *value {
                idx
            }
            else {
                std::cmp::max(idx - 1, 0)
            }
        });
        idx
    }

    fn is_sorted(&self) -> bool {
        for pos in 0..self.nb_of_positions() {
            let pos_values = &self.values_by_position[pos];
            let pos_sorted = (0..self.nb_of_items() - 1).all(|i| pos_values[i] <= pos_values[i + 1]);
            if ! pos_sorted {
                return false;
            }
        }
        true
    }
}

struct ItemValuesIter<'a, ItemData, Value> {
    chain : & 'a Chain<ItemData, Value>,
    item_idx : usize,
    position : usize,
}

impl<'a, ItemData, Value>  Iterator for ItemValuesIter<'a, ItemData, Value>
where Value : Clone
{
    type Item = Value;

    fn next(&mut self) -> Option<Self::Item> {
        if self.position >= self.chain.values_by_position.len() {
            None
        }
        else {
            let result = self.chain.values_by_position[self.position][self.item_idx].clone();
            self.position = self.position + 1;
            Some(result)
            
        }
    }
}
