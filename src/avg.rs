pub struct Avg {
    values: Vec<f64>,
    size: usize,
    position: usize,
    num_values: usize,
}

impl Avg {
    pub fn new(size: usize) -> Self {
        Avg{values: vec!{0.0; size},
            size,
            position: 0,
            num_values: 0
        }
    }

    pub fn add_value(&mut self, value: f64) -> Option<f64> {
        self.values[self.position] = value;
        self.position += 1;
        if self.position >= self.size {
            self.position = 0;
        }
        if self.num_values < self.size {
            self.num_values += 1;
        }
        if self.num_values == self.size {
            let avg: f64 = self.values.iter().sum::<f64>() / (self.size as f64);
            Some(avg)
        } else {
            None
        }
    }
}
