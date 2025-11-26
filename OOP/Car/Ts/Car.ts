interface Vehicle { start(): void }

class Car implements Vehicle {
    private engineId: string;
    constructor(engineId: string) { this.engineId = engineId; }
    start() { console.log(`Car ${this.engineId} starts!`) }
}

const v: Vehicle = new Car("ENG-001");
v.start();