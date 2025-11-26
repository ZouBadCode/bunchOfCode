package main

import "fmt"

type Vehicle interface {
	Start()
}

type Car struct {
	engineID string
}

func (c Car) Start() {
	fmt.Printf("Car %s starts!\n", c.engineID)
}

func main() {
	var v Vehicle = Car{engineID: "V8"}
	v.Start()
}
