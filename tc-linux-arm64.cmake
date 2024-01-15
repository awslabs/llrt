set(CMAKE_C_COMPILER ${CMAKE_CURRENT_LIST_DIR}/zigcc "-target aarch64-linux-musl")
set(CMAKE_CXX_COMPILER ${CMAKE_CURRENT_LIST_DIR}/zigcc "-target aarch64-linux-musl")

set(CMAKE_SYSTEM_NAME "Linux")
set(CMAKE_SYSTEM_PROCESSOR "aarch64")

set(CMAKE_AR "${CMAKE_CURRENT_LIST_DIR}/zigar")
set(CMAKE_RANLIB "${CMAKE_CURRENT_LIST_DIR}/zigranlib")

# set(CMAKE_C_COMPILER_FORCED 1)
# set(CMAKE_C_COMPILER   zig "cc -target aarch64-linux-musl")

# set(CMAKE_CXX_COMPILER zig "c++ -target aarch64-linux-musl")
# set(CMAKE_CXX_COMPILER_FORCED 1)