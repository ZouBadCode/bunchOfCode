#include<iostream>
using namespace std;

class Animal {
    public:
        void makeSound() {
            cout << "Animal sound" << endl;
        }
};

class Dog : public Animal {
    public:
        void barkloud() {
            cout << "Woof! Woof!" << endl;
        }
};

int main() {
    Dog myDog;
    myDog.makeSound(); // Inherited method from Animal class
    myDog.barkloud();  // Method from Dog class
    return 0;
}