#include <iostream>
#include <vector>
using namespace std;

int main() {
    vector<vector<int>> A = {
        {1, 2, 3},
        {4, 5, 6}
    };

    vector<vector<int>> B = {
        {7, 8, 9},
        {10, 11, 12}
    };

    vector<vector<int>> C(2, vector<int>(3));
    for (int i = 0; i < 2; ++i)
        for (int j = 0; j < 3; ++j)
            C[i][j] = A [i][j] + B[i][j];

    cout << "Resultant Matrix C:" << endl;
    for (auto &row : C) {
        for (auto &val : row) cout << val << " ";
        cout << endl;
    }

    vector<vector<int>> D = {
        {1, 2},
        {3, 4},
        {5, 6}
    };

    vector<vector<int>> result(2, vector<int>(2, 0));
    for (int i = 0; i < 2; ++i){
        for (int j = 0; j < 2; ++j){
                for (int k = 0; k < 3; ++k) {
                    result[i][j] += A[i][k] * D[k][j];
                }
            }
    }

    D = result;

    cout << "\nMatrix D = A x D:" << endl;
    for (auto &row : D) {
        for (auto &val : row) cout << val << " ";
        cout << endl;
    }

    return 0;
}