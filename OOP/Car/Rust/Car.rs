trait Vehicle { fn start(&self); }

struct Car { engine_id: String }

impl Vehicle for Car {
    fn start(&self) {
        println!("Car {} starts!", self.engine_id);
    }
}

fn main() {
    let v: Box<dyn Vehicle> = Box::new(Car { engine_id: "ENG-001".into() });
    v.start();
}