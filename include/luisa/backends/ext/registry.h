//
// Created by Mike on 5/24/2023.
//

#pragma once

#include <cstdint>

namespace luisa::compute {

enum struct CustomCommandUUID : uint32_t {

    CUSTOM_DISPATCH_EXT_BEGIN = 0x0000u,
    CUSTOM_DISPATCH = CUSTOM_DISPATCH_EXT_BEGIN,

    RASTER_EXT_BEGIN = 0x0100u,
    RASTER_DRAW_SCENE = RASTER_EXT_BEGIN,
    RASTER_CLEAR_DEPTH,

    DSTORAGE_EXT_BEGIN = 0x0200u,
    DSTORAGE_READ = DSTORAGE_EXT_BEGIN,

    DENOISER_EXT_BEGIN = 0x0300u,
    DENOISER_DENOISE = DENOISER_EXT_BEGIN,

    REGISTERED_END = 0xffffu,
};

}// namespace luisa::compute

