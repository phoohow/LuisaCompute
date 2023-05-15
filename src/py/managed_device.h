#pragma once
#include <py/ref_counter.h>
#include <runtime/device.h>
#include <vstl/meta_lib.h>
using namespace luisa::compute;
class ManagedDevice {
public:
    Device device;
    bool valid;
    ManagedDevice(Device &&device) noexcept;
    ManagedDevice(ManagedDevice &&v) noexcept;
    ManagedDevice(ManagedDevice const &) = delete;
    ~ManagedDevice() noexcept;
};