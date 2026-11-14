#include "testlib.h"
#include <string>

// Line comparison with ASCII trailing whitespace and final empty lines ignored.
// Keep internal spaces, line breaks, case, leading zeroes and non-ASCII bytes.
static std::string readLineBytes(InStream& stream) {
    std::string line;
    while (!stream.eof()) {
        char ch = stream.readChar();
        if (ch == '\n') break;
        line += ch;
    }
    while (!line.empty() &&
           (line.back() == ' ' || line.back() == '\t' || line.back() == '\r')) {
        line.pop_back();
    }
    return line;
}

int main(int argc, char* argv[]) {
    setName("compare lines, ignoring trailing ASCII whitespace and final empty lines");
    registerTestlibCmd(argc, argv);
    // testlib skips a UTF-8 BOM by default. Reopen to compare it as content.
    ouf.init(argv[2], _output);
    int line = 0;
    while (!ans.eof() && !ouf.eof()) {
        ++line;
        std::string expected = readLineBytes(ans);
        std::string actual = readLineBytes(ouf);
        if (expected != actual) {
            quitf(_wa, "line %d differs - expected: '%s', found: '%s'", line,
                  compress(expected).c_str(), compress(actual).c_str());
        }
    }
    while (!ans.eof()) {
        if (!readLineBytes(ans).empty()) quitf(_wa, "Unexpected end of participant output");
    }
    while (!ouf.eof()) {
        if (!readLineBytes(ouf).empty()) quitf(_wa, "Participant output contains extra lines");
    }
    quitf(_ok, "lines match");
}
