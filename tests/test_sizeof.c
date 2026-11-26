// TODO : re-enable this after adding typedef and use size_t instead of unsigned long
// #include <stddef.h>

int main(){
    unsigned long s = sizeof(int);
    unsigned long s2 = sizeof(s);
    return 0;
}