declare module "process" {
  global {
    namespace QuickJS {
      type Signals =
        | "SIGABRT"
        | "SIGALRM"
        | "SIGFPE"
        | "SIGHUP"
        | "SIGILL"
        | "SIGINT"
        | "SIGKILL"
        | "SIGPIPE"
        | "SIGQUIT"
        | "SIGSEGV"
        | "SIGTERM";
    }
  }
}
