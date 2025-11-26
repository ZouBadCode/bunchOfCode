trait Machine {
    fn start(&mut self);
}

pub struct CoffeeMachine {
    target_temp_c: u32,
    water_ml: u32
}

impl CoffeeMachine {
    pub fn new(target_temp_c: u32, water_ml: u32) -> Self {
        CoffeeMachine {
            target_temp_c,
            water_ml
        }
    }
    }
}

impl Machine for CoffeeMachine {
    fn start(&mut self) {
        println!(
            "Starting coffee machine to heat {}ml of water to {}°C",
            self.water_ml, self.target_temp_c
        );
    }
}

fn main() {
    let mut m = CoffeeMachine::new(90, 250);
    m.start();
}