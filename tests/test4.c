int putchar(int ch);

int main(){
    int i = 0;
    while (i < 20){
        putchar('0' + i);
        i++;
    }
    putchar('\n');
    return 0;
}