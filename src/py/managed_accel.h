#pragma once

#include <runtime/rtx/accel.h>
#include <runtime/rtx/mesh.h>
#include <py/managed_collector.h>
#include <py/py_stream.h>
#include <vstl/md5.h>
#include <core/stl/unordered_map.h>

namespace luisa::compute {

struct MeshUpdateCmd {
    AccelOption option;
    uint64_t vertex_buffer;
    size_t vertex_buffer_offset;
    size_t vertex_buffer_size;
    size_t vertex_stride;
    uint64_t triangle_buffer;
    size_t triangle_buffer_offset;
    size_t triangle_buffer_size;
};

class ManagedAccel final {
    struct Data : public vstd::IOperatorNewBase {
        ManagedCollector collector;
        Accel accel;
        struct MeshValue {
            uint64 mesh;
        };
        vstd::vector<std::pair<uint64, vstd::MD5>> meshes;
        struct MeshRef {
            uint64 mesh;
            uint64 ref_count;
        };
        struct MD5Hash {
            uint64 operator()(vstd::MD5 const &md5) const noexcept {
                return luisa::hash64(&md5, sizeof(vstd::MD5), luisa::hash64_default_seed);
            }
        };
        luisa::unordered_map<vstd::MD5, MeshRef, MD5Hash> created_mesh;

        luisa::unordered_map<uint64, MeshUpdateCmd> requireUpdateMesh;
        vstd::vector<uint64> meshDisposeList;
        Data(Accel &&accel) noexcept;
    };
    vstd::unique_ptr<Data> data;
    uint64 set_mesh(size_t index, MeshUpdateCmd const &mesh) noexcept;
    void remove_mesh(size_t index) noexcept;

public:
    Accel &GetAccel() noexcept { return data->accel; }
    ManagedAccel(Accel &&accel) noexcept;
    ManagedAccel(ManagedAccel &&) noexcept = default;
    ManagedAccel(ManagedAccel const &) = delete;
    ~ManagedAccel() noexcept;
    ManagedAccel &operator=(ManagedAccel &&) = default;
    ManagedAccel &operator=(ManagedAccel const &) = delete;
    ManagedAccel() = delete;
    void emplace(MeshUpdateCmd const &mesh, float4x4 const &transform, uint visibility_mask, bool opaque) noexcept;
    void pop_back() noexcept;
    void set(size_t idx, MeshUpdateCmd const &mesh, float4x4 const &transform, uint visibility_mask, bool opaque) noexcept;
    void update(PyStream &stream) noexcept;
};

}// namespace luisa::compute
