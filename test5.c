// TODO : do a better test harness

int putchar(int ch);

int main(){
    for (int i = 0; i < 20; i++){
        putchar('0' + i);
    }
    putchar('\n');
    return 0;
}