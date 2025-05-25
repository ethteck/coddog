extern float some_external_function(float a, float b);
int math_op_1(int a, int b);
int math_op_2(int a, int b);

int cat = 1;
int dog = 5;

int test_1(int state) {
    switch (state) {
        case 0:
            return math_op_1(cat, dog);
        case 1:
            return math_op_2(cat, dog);
        case 2:
            return some_external_function(cat, dog);
        case 3:
            return 5;
        case 4:
            return 5;
        default:
            return -1;
    }
}

int test_2(int state) {
    switch (state) {
        case 0:
            return math_op_2(cat, dog);
        case 1:
            return math_op_1(cat, dog);
        case 2:
            return some_external_function(cat, dog);
        case 3:
            return 5;
        case 4:
            return 5;
        default:
            return -1;
    }
}

int math_op_1(int a, int b) {
    return a + b + some_external_function(a, b);
}

int math_op_2(int a, int b) {
    return a - b;
}

int math_op_1_dup(int a, int b) {
    return a + b + some_external_function(a, b);
}
