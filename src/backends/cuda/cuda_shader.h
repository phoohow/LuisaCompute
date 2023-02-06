//
// Created by Mike on 2021/12/4.
//

#pragma once

#include <span>
#include <memory>

#include <core/basic_types.h>
#include <ir/ir.hpp>

namespace luisa::compute {
class ShaderDispatchCommand;
class ShaderDispatchExCommand;
}

namespace luisa::compute::cuda {

class CUDADevice;
class CUDAStream;

/**
 * @brief Shader on CUDA
 * 
 */
class CUDAShader {

protected:
    luisa::vector<ir::Binding> _captures;// only for shaders from IR

public:
    explicit CUDAShader(luisa::vector<ir::Binding> captures) noexcept
        : _captures{std::move(captures)} {}
    CUDAShader(CUDAShader &&) noexcept = delete;
    CUDAShader(const CUDAShader &) noexcept = delete;
    CUDAShader &operator=(CUDAShader &&) noexcept = delete;
    CUDAShader &operator=(const CUDAShader &) noexcept = delete;
    virtual ~CUDAShader() noexcept = default;
    /**
     * @brief Create a shader object from code
     * 
     * @param device CUDADevice
     * @param ptx kernel code
     * @param ptx_size code size
     * @param entry name of function
     * @param is_raytracing is raytracing
     * @return CUDAShader* 
     */
    [[nodiscard]] static CUDAShader *create(CUDADevice *device, uint3 block_size,
                                            const char *ptx, size_t ptx_size,
                                            const char *entry, bool is_raytracing,
                                            luisa::vector<ir::Binding>) noexcept;
    /**
     * @brief Destroy a CUDAShader
     * 
     * @param shader 
     */
    static void destroy(CUDAShader *shader) noexcept;
    /**
     * @brief Launch command to stream
     * 
     * @param stream 
     * @param command 
     */
    virtual void launch(CUDAStream *stream, const ShaderDispatchCommand *command) const noexcept = 0;
    virtual void launch(CUDAStream *stream, const ShaderDispatchExCommand *command) const noexcept = 0;
};

}
