#include <iostream>
#include <vector>
#include <string>
using namespace std;

int main() {
    string student1 = "Alice";
    string student2 = "Bob";

    vector<int> scores1;
    vector<int> scores2;

    scores1.push_back(85);
    scores1.push_back(90);
    scores1.push_back(78);

    scores2.push_back(92);
    scores2.push_back(88);
    scores2.push_back(95);

    double avg1 = (scores1[0] + scores1[1] + scores1[2]) / 3.0;
    double avg2 = (scores2[0] + scores2[1] + scores2[2]) / 3.0;

    cout << student1 << " average: " << avg1 << endl;
    cout << student2 << " average: " << avg2 << endl;

    if (avg1 > avg2)
    cout << student1 << " has a higher average " << endl;
    else if (avg2 > avg1)
    cout << student2 << " has a higher average " << endl;
    else
    cout << "Both students have the same average " << endl;

    return 0;
}