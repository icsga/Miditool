pub struct Avg {
    values: Vec<f64>,
    size: usize,
    position: usize,
    num_values: usize,
    sum: f64,
}

impl Avg {
    pub fn new(size: usize) -> Self {
        Avg{values: vec!{0.0; size},
            size,
            position: 0,
            num_values: 0,
            sum: 0.0
        }
    }

    /// Add value to the ringbuffer, return average.
    pub fn add_value(&mut self, value: f64) -> f64 {
        if self.num_values == self.size {
            self.sum -= self.values[self.position];
        }
        self.values[self.position] = value;
        self.sum += value;
        self.position += 1;
        if self.position >= self.size {
            self.position = 0;
        }
        if self.num_values < self.size {
            self.num_values += 1;
        }
        self.sum / (self.num_values as f64)
    }
}
