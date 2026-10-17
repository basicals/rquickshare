#include "MurmurHash3.h"
#include <cstdio>
#include <cstdint>
#include <string>
void emit(const std::string &s, bool last) {
 uint64_t out[2] = {0,0}; MurmurHash3_x64_128(s.data(),s.size(),0,out);
 unsigned char bloom[10] = {};
 uint32_t a=uint32_t(out[0]), b=uint32_t(out[0]>>32);
 for(uint32_t i=1;i<=5;i++){ int32_t c=int32_t(a+i*b); if(c<0)c=~c; uint32_t p=uint32_t(c)%80; bloom[p/8]|=1<<(p%8); }
 std::printf("{\"input\":\"");for(unsigned char c:s)std::printf("%02x",c);
 std::printf("\",\"low\":%llu,\"bloom\":\"",(unsigned long long)out[0]);for(auto c:bloom)std::printf("%02x",c);
 std::printf("\"}%s\n",last?"":",");
}
int main(){std::printf("[\n");for(int n:{0,1,7,8,9,15,16,17,31,32,33,128}){std::string s;for(int i=0;i<n;i++)s+=char(i);emit(s,false);}emit("hello",false);emit("NearbySharing",true);std::printf("]\n");}
