//
// Created by Mike Smith on 2021/9/24.
//

#pragma once

#include <deque>
#include <queue>
#include <map>
#include <chrono>
#include <vector>
#include <optional>

#include <asio.hpp>
#include <network/render_tile.h>
#include <network/render_config.h>

namespace luisa::compute {

class RenderBuffer;
class RenderWorkerSession;
class RenderServer;

class RenderScheduler : public std::enable_shared_from_this<RenderScheduler> {

public:
    using interval_type = std::chrono::system_clock::duration;

private:
    RenderServer *_server;
    asio::system_timer _timer;
    interval_type _interval;
    std::shared_ptr<RenderConfig> _config;
    uint _render_id;
    uint _frame_id;
    std::vector<std::shared_ptr<RenderWorkerSession>> _workers;
    std::queue<RenderTile> _tiles;
    std::queue<RenderTile> _recycled_tiles;
    std::map<uint /* frame_index */, RenderBuffer> _frames;

private:
    static void _dispatch(std::shared_ptr<RenderScheduler> self) noexcept;
    void _purge() noexcept;
    [[nodiscard]] std::optional<RenderTile> _next_tile() noexcept;

public:
    RenderScheduler(RenderServer *server, interval_type dispatch_interval) noexcept;
    [[nodiscard]] asio::io_context &context() const noexcept;
    void add(std::shared_ptr<RenderWorkerSession> worker) noexcept;
    void start(std::shared_ptr<RenderConfig> config) noexcept;
    void stop() noexcept;
    void close() noexcept;
    void recycle(RenderTile tile) noexcept;
    void accumulate(RenderTile tile, std::span<const std::byte> data) noexcept;
};

}// namespace luisa::compute
