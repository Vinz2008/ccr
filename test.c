int f(int arg){
    return 2 * arg + 2;
}

int main(){
    int a = 3;
    int b = f(a);
    if (a == 1){
        return 2;
    }
    return b;
}