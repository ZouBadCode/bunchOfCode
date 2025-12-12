#include <iostream>
#include <vector>
#include <string>
#include <algorithm>

using namespace std;

int main() {
  vector<string> valid_words;
  string word;

  while (cin >> word && word.length() >= 8) {
    valid_words.push_back(word);
  }

  cout << "Valid words:" << endl;
  for (auto it = valid_words.begin(); it != valid_words.end();         
++it) {
    cout << *it << endl;
  }

  return 0;
}