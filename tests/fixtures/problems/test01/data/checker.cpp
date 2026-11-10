#include <fstream>
#include <string>
int main(int argc,char**argv){if(argc<4)return 1;std::ifstream a(argv[2]),b(argv[3]);std::string x,y;while(a>>x){if(!(b>>y)||x!=y)return 1;}return b>>y?1:0;}
