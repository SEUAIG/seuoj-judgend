#include "testlib.h"

int main(int argc, char* argv[]) {
    registerTestlibCmd(argc, argv);

    int pans = ouf.readInt();
    int jans = ans.readInt();

    if (pans == jans)
        quitf(_ok, "");
    else
        quitf(_wa, "expected = %d, found = %d", jans, pans);
}
