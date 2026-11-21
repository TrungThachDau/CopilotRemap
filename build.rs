fn main() {
    // Stages a private Windows App Runtime next to the binaries so the WinUI 3
    // settings window works without a separately installed runtime.
    windows_reactor_setup::as_self_contained();
}