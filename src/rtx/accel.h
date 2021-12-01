//
// Created by Mike Smith on 2021/6/24.
//

#pragma once

#include <core/basic_types.h>
#include <core/allocator.h>

#include <rtx/ray.h>
#include <rtx/hit.h>
#include <rtx/mesh.h>

namespace luisa::compute {

class Accel final {

private:
    Device::Handle _device;
    uint64_t _handle{};
    luisa::unordered_set<const Mesh *> _meshes;
    size_t _size{};
    bool _requires_rebuild{true};

private:
    friend class Device;
    friend class Mesh;
    explicit Accel(Device::Interface *device, AccelBuildHint hint = AccelBuildHint::FAST_TRACE) noexcept;
    void _set_requires_rebuild() noexcept;
    void _replace(const Mesh *prev, const Mesh *curr) noexcept;
    void _remove(const Mesh *m) noexcept;
    void _destroy() noexcept;

public:
    Accel() noexcept = default;
    ~Accel() noexcept;
    Accel(Accel &&another) noexcept;
    Accel(const Accel &) noexcept = delete;
    Accel &operator=(Accel &&rhs) noexcept;
    Accel &operator=(const Accel &) noexcept = delete;
    [[nodiscard]] explicit operator bool() const noexcept { return _device != nullptr; }
    [[nodiscard]] auto device() const noexcept { return _device.get(); }
    [[nodiscard]] auto handle() const noexcept { return _handle; }
    [[nodiscard]] auto size() const noexcept { return _size; }
    Accel &emplace_back(const Mesh &mesh, float4x4 transform = luisa::make_float4x4(1.0f)) noexcept;
    Accel &set_transform(size_t index, float4x4 transform) noexcept;
    [[nodiscard]] Command *update() noexcept;
    [[nodiscard]] Command *build() noexcept;

    // shader functions
    [[nodiscard]] Var<Hit> trace_closest(Expr<Ray> ray) const noexcept;
    [[nodiscard]] Var<bool> trace_any(Expr<Ray> ray) const noexcept;
};

template<>
struct Expr<Accel> {

private:
    const RefExpr *_expression{nullptr};

public:
    explicit Expr(const RefExpr *expr) noexcept
        : _expression{expr} {}
    explicit Expr(const Accel &accel) noexcept
        : _expression{detail::FunctionBuilder::current()->accel_binding(accel.handle())} {}
    [[nodiscard]] auto expression() const noexcept { return _expression; }
    [[nodiscard]] auto trace_closest(Expr<Ray> ray) const noexcept {
        return def<Hit>(
            detail::FunctionBuilder::current()->call(
                Type::of<Hit>(), CallOp::TRACE_CLOSEST,
                {_expression, ray.expression()}));
    }
    [[nodiscard]] auto trace_any(Expr<Ray> ray) const noexcept {
        return def<bool>(
            detail::FunctionBuilder::current()->call(
                Type::of<bool>(), CallOp::TRACE_ANY,
                {_expression, ray.expression()}));
    }
};

template<>
struct Var<Accel> : public Expr<Accel> {
    explicit Var(detail::ArgumentCreation) noexcept
        : Expr<Accel>{
              detail::FunctionBuilder::current()->accel()} {}
    Var(Var &&) noexcept = default;
    Var(const Var &) noexcept = delete;
    Var &operator=(Var &&) noexcept = delete;
    Var &operator=(const Var &) noexcept = delete;
};

using AccelVar = Var<Accel>;

}// namespace luisa::compute
