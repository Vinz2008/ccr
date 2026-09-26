int f(int arg){
    return 2 * arg + 2;
}

int main(int argc){
    int a = 3;
    int b = f(a);
    if (argc == 2){
        return 2;
    }
    return b;
}