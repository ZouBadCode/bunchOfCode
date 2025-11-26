#include <iostream>
#include <string>
#include <cctype>

int main() {
    std::string line;
    std::getline(std::cin, line);  // 讀入整行

    int total = line.size();
    int letters = 0, digits = 0, spaces = 0;

    for (char c : line) {
        if (std::isalpha(static_cast<unsigned char>(c))) letters++;
        else if (std::isdigit(static_cast<unsigned char>(c))) digits++;
        else if (std::isspace(static_cast<unsigned char>(c))) spaces++;
    }

    std::cout << "Total characters: " << total << "\n";
    std::cout << "Letters: " << letters << ", Digits: " << digits << ", Spaces: " << spaces << "\n";

    // 步驟①：將逗號改成空白
    for (char &c : line) {
        if (c == ',') c = ' ';
    }

    // 步驟②：壓縮多餘空白
    std::string cleaned;
    bool in_space = false;
    for (char c : line) {
        if (std::isspace(static_cast<unsigned char>(c))) {
            if (!in_space) {
                cleaned += ' ';
                in_space = true;
            }
        } else {
            cleaned += c;
            in_space = false;
        }
    }

    // 去除開頭與結尾空白
    if (!cleaned.empty() && cleaned.front() == ' ') cleaned.erase(cleaned.begin());
    if (!cleaned.empty() && cleaned.back() == ' ') cleaned.pop_back();

    // 步驟③：輸出最終行
    std::cout << "Final line: " << cleaned << "\n";

    return 0;
}
