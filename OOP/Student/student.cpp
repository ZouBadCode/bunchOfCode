#include <iostream>
#include <string>
#include <vector>
#include <numeric> // for std::accumulate

class Student {
private:
    std::string name_;
    int id_;
    std::vector<double> scores_;

public:
    // Constructor
    Student(const std::string& name, int id)
        : name_(name), id_(id) {}

    // Getters
    std::string name() const {
        return name_;
    }

    int id() const {
        return id_;
    }

    double average() const {
        if (scores_.empty()) {
            return 0.0;
        }
        double sum = std::accumulate(scores_.begin(), scores_.end(), 0.0);
        return sum / scores_.size();
    }

    // Modifier
    void addScore(double s) {
        scores_.push_back(s);
    }
};

// Non-member helper function
void printReport(const Student& s) {
    std::cout << "Student Name: " << s.name() << std::endl;
    std::cout << "ID          : " << s.id()   << std::endl;
    std::cout << "Average     : " << s.average() << std::endl;
}

int main() {
    Student stu("Alice", 12345);
    stu.addScore(85.5);
    stu.addScore(92.0);
    stu.addScore(78.5);

    printReport(stu);
    return 0;
}
