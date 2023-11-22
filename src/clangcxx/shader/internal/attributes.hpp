#pragma once
#define ignore clang::annotate("luisa-shader", "ignore")
#define builtin(name) clang::annotate("luisa-shader", "builtin", (name))
#define callop(name) clang::annotate("luisa-shader", "callop", (name))
#define expr(name) clang::annotate("luisa-shader", "callop", (name))
#define kernel_1d(x) clang::annotate("luisa-shader", "kernel_1d", (x))
#define kernel_2d(x, y) clang::annotate("luisa-shader", "kernel_2d", (x), (y))
#define kernel_3d(x, y, z) clang::annotate("luisa-shader", "kernel_3d", (x), (y), (z))

#define trait struct [[ignore]]