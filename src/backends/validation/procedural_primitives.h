#pragma once
#include "rw_resource.h"
namespace lc::validation {
class Buffer;
class ProceduralPrimitives : public RWResource {
public:
    Buffer *bbox{};
    ProceduralPrimitives(uint64_t handle)
        : RWResource(handle, Tag::PROCEDURAL_PRIMITIVE, false) {}
    void set(Stream *stream, Usage usage) override;
};
}// namespace lc::validation