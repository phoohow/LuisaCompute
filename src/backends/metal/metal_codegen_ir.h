//
// Created by Mike Smith on 2023/4/15.
//

#pragma once

#include <ir/ir2ast.h>

namespace luisa::compute::metal {

class MetalCodegenIR {

public:
    [[nodiscard]] static size_t type_size_bytes(const ir::Type *type) noexcept;
};

}// namespace luisa::compute::metal
