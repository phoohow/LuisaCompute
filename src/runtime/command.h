//
// Created by Mike Smith on 2021/3/3.
//

#pragma once

#include <core/basic_types.h>
#include <ast/variable.h>
#include <ast/function.h>
#include <runtime/pixel.h>
#include <ast/function_builder.h>
#include <runtime/stream_tag.h>
#include <raster/viewport.h>
#include <runtime/custom_struct.h>
#include <runtime/sampler.h>
#include <runtime/arguments.h>
#include <rust/luisa_compute_api_types/bindings.h>

namespace luisa::compute {

class CmdDeser;
class CmdSer;
class RasterMesh;

#define LUISA_COMPUTE_RUNTIME_COMMANDS   \
    BufferUploadCommand,                 \
        BufferDownloadCommand,           \
        BufferCopyCommand,               \
        BufferToTextureCopyCommand,      \
        ShaderDispatchCommand,           \
        ShaderDispatchExCommand,         \
        TextureUploadCommand,            \
        TextureDownloadCommand,          \
        TextureCopyCommand,              \
        TextureToBufferCopyCommand,      \
        AccelBuildCommand,               \
        MeshBuildCommand,                \
        ProceduralPrimitiveBuildCommand, \
        BindlessArrayUpdateCommand,      \
        CustomCommand,                   \
        DrawRasterSceneCommand,          \
        ClearDepthCommand

#define LUISA_MAKE_COMMAND_FWD_DECL(CMD) class CMD;
LUISA_MAP(LUISA_MAKE_COMMAND_FWD_DECL, LUISA_COMPUTE_RUNTIME_COMMANDS)
#undef LUISA_MAKE_COMMAND_FWD_DECL

struct CommandVisitor {
#define LUISA_MAKE_COMMAND_VISITOR_INTERFACE(CMD) \
    virtual void visit(const CMD *) noexcept = 0;
    LUISA_MAP(LUISA_MAKE_COMMAND_VISITOR_INTERFACE, LUISA_COMPUTE_RUNTIME_COMMANDS)
#undef LUISA_MAKE_COMMAND_VISITOR_INTERFACE
};

struct MutableCommandVisitor {
#define LUISA_MAKE_COMMAND_VISITOR_INTERFACE(CMD) \
    virtual void visit(CMD *) noexcept = 0;
    LUISA_MAP(LUISA_MAKE_COMMAND_VISITOR_INTERFACE, LUISA_COMPUTE_RUNTIME_COMMANDS)
#undef LUISA_MAKE_COMMAND_VISITOR_INTERFACE
};

class Command;
class CommandList;

#define LUISA_MAKE_COMMAND_COMMON_CREATE(Cmd)                        \
    template<typename... Args>                                       \
        requires(std::is_constructible_v<Cmd, Args && ...>)          \
    [[nodiscard]] static auto create(Args &&...args) noexcept {      \
        return luisa::make_unique<Cmd>(std::forward<Args>(args)...); \
    }

#define LUISA_MAKE_COMMAND_COMMON_ACCEPT(Cmd)                                             \
    void accept(CommandVisitor &visitor) const noexcept override { visitor.visit(this); } \
    void accept(MutableCommandVisitor &visitor) noexcept override { visitor.visit(this); }

#define LUISA_MAKE_COMMAND_COMMON(Cmd, Type) \
    friend class CmdDeser;                   \
    friend class CmdSer;                     \
    LUISA_MAKE_COMMAND_COMMON_CREATE(Cmd)    \
    LUISA_MAKE_COMMAND_COMMON_ACCEPT(Cmd)    \
    StreamTag stream_tag() const noexcept override { return Type; }

class Command {

public:
    enum struct Tag {
#define LUISA_MAKE_COMMAND_TAG(Cmd) E##Cmd,
        LUISA_MAP(LUISA_MAKE_COMMAND_TAG, LUISA_COMPUTE_RUNTIME_COMMANDS)
#undef LUISA_MAKE_COMMAND_TAG
    };

private:
    Tag _tag;

public:
    explicit Command(Tag tag) noexcept : _tag(tag) {}
    virtual ~Command() noexcept = default;
    virtual void accept(CommandVisitor &visitor) const noexcept = 0;
    virtual void accept(MutableCommandVisitor &visitor) noexcept = 0;
    [[nodiscard]] auto tag() const noexcept { return _tag; }
    [[nodiscard]] virtual StreamTag stream_tag() const noexcept = 0;
};

class ShaderDispatchCommandBase : public Command {

private:
    luisa::vector<std::byte> _argument_buffer;
    uint32_t _argument_count{0u};

protected:
    ShaderDispatchCommandBase(luisa::vector<std::byte> &&argument_buffer, uint32_t argument_count, Tag tag)
        : Command{tag},
          _argument_buffer{std::move(argument_buffer)},
          _argument_count{argument_count} {}

public:
    [[noreturn]] LC_RUNTIME_API static void _error_invalid_argument() noexcept;
    template<typename Visit>
    void decode(Visit &&visit) const noexcept {
        auto p = _argument_buffer.data();
        auto end = _argument_buffer.data() + _argument_buffer.size();
        while (p < end) {
            Argument argument{};
            std::memcpy(&argument, p, sizeof(Argument));
            switch (argument.tag) {
                case Argument::Tag::BUFFER: {
                    BufferArgument buffer_argument{};
                    std::memcpy(&buffer_argument, p, sizeof(BufferArgument));
                    visit(buffer_argument);
                    p += sizeof(BufferArgument);
                    break;
                }
                case Argument::Tag::TEXTURE: {
                    TextureArgument texture_argument{};
                    std::memcpy(&texture_argument, p, sizeof(TextureArgument));
                    visit(texture_argument);
                    p += sizeof(TextureArgument);
                    break;
                }
                case Argument::Tag::UNIFORM: {
                    UniformArgumentHead head{};
                    std::memcpy(&head, p, sizeof(UniformArgumentHead));
                    p += sizeof(UniformArgumentHead);
                    visit(UniformArgument{head, p});
                    p += head.size;
                    break;
                }
                case Argument::Tag::BINDLESS_ARRAY: {
                    BindlessArrayArgument bindless_array_argument;
                    std::memcpy(&bindless_array_argument, p, sizeof(BindlessArrayArgument));
                    visit(bindless_array_argument);
                    p += sizeof(BindlessArrayArgument);
                    break;
                }
                case Argument::Tag::ACCEL: {
                    AccelArgument accel_argument;
                    std::memcpy(&accel_argument, p, sizeof(AccelArgument));
                    visit(accel_argument);
                    p += sizeof(AccelArgument);
                    break;
                }
                default: {
                    _error_invalid_argument();// no return
                }
            }
        }
    }
};

class ShaderDispatchCommand final : public ShaderDispatchCommandBase {
    friend class ComputeDispatchCmdEncoder;
    ShaderDispatchCommand(luisa::vector<std::byte> &&argument_buffer, uint32_t argument_count)
        : ShaderDispatchCommandBase{std::move(argument_buffer), argument_count, Tag::EShaderDispatchCommand} {}
    uint64_t _handle;
    luisa::variant<
        uint3,
        IndirectDispatchArg>
        _dispatch_size;

public:
    ShaderDispatchCommand(ShaderDispatchCommand const &) = delete;
    ShaderDispatchCommand(ShaderDispatchCommand &&) = default;
    [[nodiscard]] auto handle() const noexcept { return _handle; }
    [[nodiscard]] auto const &dispatch_size() const noexcept { return _dispatch_size; }
    LUISA_MAKE_COMMAND_COMMON(ShaderDispatchCommand, StreamTag::COMPUTE)
};

class LC_RUNTIME_API DrawRasterSceneCommand final : public ShaderDispatchCommandBase {
    friend class RasterDispatchCmdEncoder;
    DrawRasterSceneCommand(luisa::vector<std::byte> &&argument_buffer, uint32_t argument_count);
    uint64_t _handle;
    TextureArgument _rtv_texs[8];
    size_t _rtv_count;
    TextureArgument _dsv_tex;
    luisa::vector<RasterMesh> _scene;
    Viewport _viewport;

public:
    DrawRasterSceneCommand(DrawRasterSceneCommand const &) = delete;
    DrawRasterSceneCommand(DrawRasterSceneCommand &&);
    ~DrawRasterSceneCommand() noexcept;
    [[nodiscard]] auto handle() const noexcept { return _handle; }
    [[nodiscard]] auto rtv_texs() const noexcept { return luisa::span<const TextureArgument>{_rtv_texs, _rtv_count}; }
    [[nodiscard]] auto const &dsv_tex() const noexcept { return _dsv_tex; }
    [[nodiscard]] luisa::span<const RasterMesh> scene() const noexcept;
    [[nodiscard]] auto viewport() const noexcept { return _viewport; }
    LUISA_MAKE_COMMAND_COMMON(DrawRasterSceneCommand, StreamTag::GRAPHICS)
};

class BufferUploadCommand final : public Command {

private:
    uint64_t _handle{};
    size_t _offset{};
    size_t _size{};
    const void *_data{};

private:
    BufferUploadCommand() noexcept
        : Command{Command::Tag::EBufferUploadCommand} {}

public:
    BufferUploadCommand(uint64_t handle,
                        size_t offset_bytes,
                        size_t size_bytes,
                        const void *data) noexcept
        : Command{Command::Tag::EBufferUploadCommand},
          _handle{handle}, _offset{offset_bytes}, _size{size_bytes}, _data{data} {}
    [[nodiscard]] auto handle() const noexcept { return _handle; }
    [[nodiscard]] auto offset() const noexcept { return _offset; }
    [[nodiscard]] auto size() const noexcept { return _size; }
    [[nodiscard]] auto data() const noexcept { return _data; }
    LUISA_MAKE_COMMAND_COMMON(BufferUploadCommand, StreamTag::COPY)
};

class BufferDownloadCommand final : public Command {

private:
    uint64_t _handle{};
    size_t _offset{};
    size_t _size{};
    void *_data{};

private:
    BufferDownloadCommand() noexcept
        : Command{Command::Tag::EBufferDownloadCommand} {}

public:
    BufferDownloadCommand(uint64_t handle, size_t offset_bytes, size_t size_bytes, void *data) noexcept
        : Command{Command::Tag::EBufferDownloadCommand},
          _handle{handle}, _offset{offset_bytes}, _size{size_bytes}, _data{data} {}
    [[nodiscard]] auto handle() const noexcept { return _handle; }
    [[nodiscard]] auto offset() const noexcept { return _offset; }
    [[nodiscard]] auto size() const noexcept { return _size; }
    [[nodiscard]] auto data() const noexcept { return _data; }
    LUISA_MAKE_COMMAND_COMMON(BufferDownloadCommand, StreamTag::COPY)
};

class BufferCopyCommand final : public Command {

private:
    uint64_t _src_handle{};
    uint64_t _dst_handle{};
    size_t _src_offset{};
    size_t _dst_offset{};
    size_t _size{};

private:
    BufferCopyCommand() noexcept
        : Command{Command::Tag::EBufferCopyCommand} {}

public:
    BufferCopyCommand(uint64_t src, uint64_t dst, size_t src_offset, size_t dst_offset, size_t size) noexcept
        : Command{Command::Tag::EBufferCopyCommand},
          _src_handle{src}, _dst_handle{dst},
          _src_offset{src_offset}, _dst_offset{dst_offset}, _size{size} {}
    [[nodiscard]] auto src_handle() const noexcept { return _src_handle; }
    [[nodiscard]] auto dst_handle() const noexcept { return _dst_handle; }
    [[nodiscard]] auto src_offset() const noexcept { return _src_offset; }
    [[nodiscard]] auto dst_offset() const noexcept { return _dst_offset; }
    [[nodiscard]] auto size() const noexcept { return _size; }
    LUISA_MAKE_COMMAND_COMMON(BufferCopyCommand, StreamTag::COPY)
};

class BufferToTextureCopyCommand final : public Command {

private:
    uint64_t _buffer_handle{};
    size_t _buffer_offset{};
    uint64_t _texture_handle{};
    PixelStorage _pixel_storage{};
    uint _texture_level{};
    uint _texture_size[3]{};

private:
    BufferToTextureCopyCommand() noexcept
        : Command{Command::Tag::EBufferToTextureCopyCommand} {}

public:
    BufferToTextureCopyCommand(uint64_t buffer, size_t buffer_offset,
                               uint64_t texture, PixelStorage storage,
                               uint level, uint3 size) noexcept
        : Command{Command::Tag::EBufferToTextureCopyCommand},
          _buffer_handle{buffer}, _buffer_offset{buffer_offset},
          _texture_handle{texture}, _pixel_storage{storage}, _texture_level{level},
          _texture_size{size.x, size.y, size.z} {}
    [[nodiscard]] auto buffer() const noexcept { return _buffer_handle; }
    [[nodiscard]] auto buffer_offset() const noexcept { return _buffer_offset; }
    [[nodiscard]] auto texture() const noexcept { return _texture_handle; }
    [[nodiscard]] auto storage() const noexcept { return _pixel_storage; }
    [[nodiscard]] auto level() const noexcept { return _texture_level; }
    [[nodiscard]] auto size() const noexcept { return uint3(_texture_size[0], _texture_size[1], _texture_size[2]); }
    LUISA_MAKE_COMMAND_COMMON(BufferToTextureCopyCommand, StreamTag::COPY)
};

class TextureToBufferCopyCommand final : public Command {

private:
    uint64_t _buffer_handle{};
    size_t _buffer_offset{};
    uint64_t _texture_handle{};
    PixelStorage _pixel_storage{};
    uint _texture_level{};
    uint _texture_size[3]{};

private:
    TextureToBufferCopyCommand() noexcept
        : Command{Command::Tag::ETextureToBufferCopyCommand} {}

public:
    TextureToBufferCopyCommand(uint64_t buffer, size_t buffer_offset,
                               uint64_t texture, PixelStorage storage,
                               uint level, uint3 size) noexcept
        : Command{Command::Tag::ETextureToBufferCopyCommand},
          _buffer_handle{buffer}, _buffer_offset{buffer_offset},
          _texture_handle{texture}, _pixel_storage{storage}, _texture_level{level},
          _texture_size{size.x, size.y, size.z} {}
    [[nodiscard]] auto buffer() const noexcept { return _buffer_handle; }
    [[nodiscard]] auto buffer_offset() const noexcept { return _buffer_offset; }
    [[nodiscard]] auto texture() const noexcept { return _texture_handle; }
    [[nodiscard]] auto storage() const noexcept { return _pixel_storage; }
    [[nodiscard]] auto level() const noexcept { return _texture_level; }
    [[nodiscard]] auto size() const noexcept { return uint3(_texture_size[0], _texture_size[1], _texture_size[2]); }
    LUISA_MAKE_COMMAND_COMMON(TextureToBufferCopyCommand, StreamTag::COPY)
};

class TextureCopyCommand final : public Command {

private:
    PixelStorage _storage{};
    uint64_t _src_handle{};
    uint64_t _dst_handle{};
    uint _size[3]{};
    uint _src_level{};
    uint _dst_level{};

private:
    TextureCopyCommand() noexcept
        : Command{Command::Tag::ETextureCopyCommand} {}

public:
    TextureCopyCommand(PixelStorage storage, uint64_t src_handle, uint64_t dst_handle,
                       uint src_level, uint dst_level, uint3 size) noexcept
        : Command{Command::Tag::ETextureCopyCommand},
          _storage{storage}, _src_handle{src_handle}, _dst_handle{dst_handle},
          _size{size.x, size.y, size.z}, _src_level{src_level}, _dst_level{dst_level} {}
    [[nodiscard]] auto storage() const noexcept { return _storage; }
    [[nodiscard]] auto src_handle() const noexcept { return _src_handle; }
    [[nodiscard]] auto dst_handle() const noexcept { return _dst_handle; }
    [[nodiscard]] auto size() const noexcept { return uint3(_size[0], _size[1], _size[2]); }
    [[nodiscard]] auto src_level() const noexcept { return _src_level; }
    [[nodiscard]] auto dst_level() const noexcept { return _dst_level; }
    LUISA_MAKE_COMMAND_COMMON(TextureCopyCommand, StreamTag::COPY)
};

class TextureUploadCommand final : public Command {

private:
    uint64_t _handle{};
    PixelStorage _storage{};
    uint _level{};
    uint _size[3]{};
    const void *_data{};

private:
    TextureUploadCommand() noexcept
        : Command{Command::Tag::ETextureUploadCommand} {}

public:
    TextureUploadCommand(uint64_t handle, PixelStorage storage,
                         uint level, uint3 size, const void *data) noexcept
        : Command{Command::Tag::ETextureUploadCommand},
          _handle{handle}, _storage{storage}, _level{level},
          _size{size.x, size.y, size.z}, _data{data} {}
    [[nodiscard]] auto handle() const noexcept { return _handle; }
    [[nodiscard]] auto storage() const noexcept { return _storage; }
    [[nodiscard]] auto level() const noexcept { return _level; }
    [[nodiscard]] auto size() const noexcept { return uint3(_size[0], _size[1], _size[2]); }
    [[nodiscard]] auto data() const noexcept { return _data; }
    LUISA_MAKE_COMMAND_COMMON(TextureUploadCommand, StreamTag::COPY)
};

class TextureDownloadCommand final : public Command {

private:
    uint64_t _handle{};
    PixelStorage _storage{};
    uint _level{};
    uint _size[3]{};
    void *_data{};

private:
    TextureDownloadCommand() noexcept
        : Command{Command::Tag::ETextureDownloadCommand} {}

public:
    TextureDownloadCommand(uint64_t handle, PixelStorage storage,
                           uint level, uint3 size, void *data) noexcept
        : Command{Command::Tag::ETextureDownloadCommand},
          _handle{handle}, _storage{storage}, _level{level},
          _size{size.x, size.y, size.z}, _data{data} {}
    [[nodiscard]] auto handle() const noexcept { return _handle; }
    [[nodiscard]] auto storage() const noexcept { return _storage; }
    [[nodiscard]] auto level() const noexcept { return _level; }
    [[nodiscard]] auto size() const noexcept { return uint3(_size[0], _size[1], _size[2]); }
    [[nodiscard]] auto data() const noexcept { return _data; }
    LUISA_MAKE_COMMAND_COMMON(TextureDownloadCommand, StreamTag::COPY)
};

enum struct AccelBuildRequest : uint8_t {
    PREFER_UPDATE,
    FORCE_BUILD,
};

class MeshBuildCommand final : public Command {

private:
    uint64_t _handle{};
    AccelBuildRequest _request{};

private:
    MeshBuildCommand() noexcept
        : Command{Command::Tag::EMeshBuildCommand} {}

public:
    MeshBuildCommand(uint64_t handle, AccelBuildRequest request) noexcept
        : Command{Command::Tag::EMeshBuildCommand}, _handle{handle}, _request{request} {
    }
    [[nodiscard]] auto handle() const noexcept { return _handle; }
    [[nodiscard]] auto request() const noexcept { return _request; }
    LUISA_MAKE_COMMAND_COMMON(MeshBuildCommand, StreamTag::COMPUTE)
};

class ProceduralPrimitiveBuildCommand final : public Command {

private:
    uint64_t _handle{};
    AccelBuildRequest _request{};

public:
    ProceduralPrimitiveBuildCommand(uint64_t handle, AccelBuildRequest request) noexcept
        : Command{Command::Tag::EProceduralPrimitiveBuildCommand},
          _handle{handle}, _request{request} {}
    [[nodiscard]] auto handle() const noexcept { return _handle; }
    [[nodiscard]] auto request() const noexcept { return _request; }
    LUISA_MAKE_COMMAND_COMMON(ProceduralPrimitiveBuildCommand, StreamTag::COMPUTE)
};

class AccelBuildCommand final : public Command {

public:
    struct alignas(16) Modification {

        // flags
        static constexpr auto flag_mesh = 1u << 0u;
        static constexpr auto flag_transform = 1u << 1u;
        static constexpr auto flag_visibility_on = 1u << 2u;
        static constexpr auto flag_visibility_off = 1u << 3u;
        static constexpr auto flag_opaque_on = 1u << 4u;
        static constexpr auto flag_opaque_off = 1u << 5u;
        static constexpr auto flag_visibility = flag_visibility_on | flag_visibility_off;
        static constexpr auto flag_opaque = flag_opaque_on | flag_opaque_off;

        // members
        uint index{};
        uint flags{};
        uint64_t mesh{};
        float affine[12]{};

        // ctor
        Modification() noexcept = default;
        explicit Modification(uint index) noexcept : index{index} {}

        // encode interfaces
        void set_transform(float4x4 m) noexcept {
            affine[0] = m[0][0];
            affine[1] = m[1][0];
            affine[2] = m[2][0];
            affine[3] = m[3][0];
            affine[4] = m[0][1];
            affine[5] = m[1][1];
            affine[6] = m[2][1];
            affine[7] = m[3][1];
            affine[8] = m[0][2];
            affine[9] = m[1][2];
            affine[10] = m[2][2];
            affine[11] = m[3][2];
            flags |= flag_transform;
        }
        void set_visibility(bool vis) noexcept {
            flags &= ~flag_visibility;// clear old visibility flags
            flags |= vis ? flag_visibility_on : flag_visibility_off;
        }
        void set_opaque(bool opaque) noexcept {
            flags &= ~flag_opaque;// clear old visibility flags
            flags |= opaque ? flag_opaque_on : flag_opaque_off;
        }
        void set_mesh(uint64_t handle) noexcept {
            mesh = handle;
            flags |= flag_mesh;
        }
    };

private:
    uint64_t _handle;
    uint32_t _instance_count;
    AccelBuildRequest _request;
    luisa::vector<Modification> _modifications;
    bool _build_accel;

public:
    AccelBuildCommand(uint64_t handle, uint32_t instance_count,
                      AccelBuildRequest request,
                      luisa::vector<Modification> modifications,
                      bool build_accel) noexcept
        : Command{Command::Tag::EAccelBuildCommand},
          _handle{handle}, _instance_count{instance_count},
          _build_accel{build_accel}, _request{request}, _modifications{std::move(modifications)} {}
    [[nodiscard]] auto handle() const noexcept { return _handle; }
    [[nodiscard]] auto request() const noexcept { return _request; }
    [[nodiscard]] auto instance_count() const noexcept { return _instance_count; }
    [[nodiscard]] auto modifications() const noexcept { return luisa::span{_modifications}; }
    [[nodiscard]] auto build_accel() const noexcept { return _build_accel; }
    LUISA_MAKE_COMMAND_COMMON(AccelBuildCommand, StreamTag::COMPUTE)
};

class BindlessArrayUpdateCommand final : public Command {

public:
    struct Modification {

        enum struct Operation : uint {
            NONE,
            EMPLACE,
            REMOVE,
        };

        struct Buffer {
            uint64_t handle;
            size_t offset_bytes;
            Operation op;
            Buffer() noexcept
                : handle{0u}, offset_bytes{0u}, op{Operation::NONE} {}
            Buffer(uint64_t handle, size_t offset_bytes, Operation op) noexcept
                : handle{handle}, offset_bytes{offset_bytes}, op{op} {}
            [[nodiscard]] static auto emplace(uint64_t handle, size_t offset_bytes) noexcept {
                return Buffer{handle, offset_bytes, Operation::EMPLACE};
            }
            [[nodiscard]] static auto remove() noexcept {
                return Buffer{0u, 0u, Operation::REMOVE};
            }
        };

        struct Texture {
            uint64_t handle;
            Sampler sampler;
            Operation op;
            Texture() noexcept
                : handle{0u}, sampler{}, op{Operation::NONE} {}
            Texture(uint64_t handle, Sampler sampler, Operation op) noexcept
                : handle{handle}, sampler{sampler}, op{op} {}
            [[nodiscard]] static auto emplace(uint64_t handle, Sampler sampler) noexcept {
                return Texture{handle, sampler, Operation::EMPLACE};
            }
            [[nodiscard]] static auto remove() noexcept {
                return Texture{0u, Sampler{}, Operation::REMOVE};
            }
        };

        size_t slot;
        Buffer buffer;
        Texture tex2d;
        Texture tex3d;

        explicit Modification(size_t slot) noexcept
            : slot{slot}, buffer{}, tex2d{}, tex3d{} {}
    };

    static_assert(sizeof(Modification) == 64u);

private:
    uint64_t _handle;
    luisa::vector<Modification> _modifications;

public:
    BindlessArrayUpdateCommand(uint64_t handle,
                               luisa::vector<Modification> mods) noexcept
        : Command{Command::Tag::EBindlessArrayUpdateCommand},
          _handle{handle}, _modifications{std::move(mods)} {}
    [[nodiscard]] auto handle() const noexcept { return _handle; }
    [[nodiscard]] luisa::span<const Modification> modifications() const noexcept { return _modifications; }
    LUISA_MAKE_COMMAND_COMMON(BindlessArrayUpdateCommand, StreamTag::COPY)
};

class ClearDepthCommand final : public Command {
    uint64_t _handle;
    float _value;

public:
    explicit ClearDepthCommand(uint64_t handle, float value) noexcept
        : Command{Command::Tag::EClearDepthCommand}, _handle{handle}, _value(value) {}
    [[nodiscard]] auto handle() const noexcept { return _handle; }
    [[nodiscard]] auto value() const noexcept { return _value; }

    LUISA_MAKE_COMMAND_COMMON(ClearDepthCommand, StreamTag::GRAPHICS)
};

class CustomCommand final : public Command {

public:
    struct BufferView {
        uint64_t handle;
        uint64_t start_byte;
        uint64_t size_byte;
    };

    struct TextureView {
        uint64_t handle;
        uint64_t start_mip;
        uint64_t size_mip;
    };

    struct MeshView {
        uint64_t handle;
    };

    struct AccelView {
        uint64_t handle;
    };

    struct BindlessView {
        uint64_t handle;
    };

    struct ResourceBinding {
        luisa::variant<
            BufferView,
            TextureView,
            MeshView,
            AccelView,
            BindlessView>
            resource_view;
        luisa::string name;
        Usage usage;
    };

private:
    luisa::vector<ResourceBinding> _resources;
    luisa::string _name;
    StreamTag _stream_tag;

public:
    explicit CustomCommand(luisa::vector<ResourceBinding> &&resources, luisa::string &&name, StreamTag stream_tag) noexcept
        : Command(Command::Tag::ECustomCommand), _resources(std::move(resources)), _name(std::move(name)), _stream_tag(stream_tag) {}
    [[nodiscard]] auto resources() const noexcept { return luisa::span{_resources}; }
    [[nodiscard]] auto name() const noexcept { return luisa::string_view{_name}; }
    LUISA_MAKE_COMMAND_COMMON(CustomCommand, _stream_tag)
};

#undef LUISA_MAKE_COMMAND_COMMON_CREATE
#undef LUISA_MAKE_COMMAND_COMMON_ACCEPT
#undef LUISA_MAKE_COMMAND_COMMON_RECYCLE
#undef LUISA_MAKE_COMMAND_COMMON

}// namespace luisa::compute
