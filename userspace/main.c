__attribute__((force_align_arg_pointer))
void _start() {
    asm("ecall");
    __builtin_unreachable();
}