int f(int arg){
    return 2 * arg + 4 + (-2);
}

int putchar(int ch);

int main(int argc, char** argv){
    int a = 3;
    int b = f(a);
    putchar('a');
    return b;
}