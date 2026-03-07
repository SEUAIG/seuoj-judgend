#include "testlib.h"

int main(int argc, char* argv[]) {
    registerTestlibCmd(argc, argv);

    int pans = ouf.readInt();
    int jans = ans.readInt();

    if (pans == jans)
        quitf(_ok, "");
    else
        quitp(0.5, "Expected %d, found %d, but I'm good, so i will give you half points", jans, pans);
}
