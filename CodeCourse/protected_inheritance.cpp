#include<iostream>
using namespace std;

class Phone {
    public:
        void turnOn() {
            cout << "Turning on the phone..." << endl;
        }
};

class Smartphone : protected Phone {
    public:
        void boot() {
            makeCall();
            cout << "Smartphone is booting up!" << endl;
        }
};

int main() {
    Smartphone myPhone;
    myPhone.boot();
    //p.turnOn(); // error
    return 0;
}