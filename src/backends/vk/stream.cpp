#include "stream.h"
#include "device.h"
#include <luisa/core/logging.h>
#include "log.h"
namespace lc::vk {
static size_t TEMP_SIZE = 1024ull * 1024ull;
Stream::Stream(Device *device, StreamTag tag)
    : Resource{device},
      _evt(device),
      _thd([this]() {
          while (_enabled) {
              while (auto p = _exec.pop()) {
                  p->visit(
                      [&]<typename T>(T const &t) {
                          if constexpr (std::is_same_v<T, Callbacks>) {
                              for (auto &i : t) {
                                  i();
                              }
                          } else if constexpr (std::is_same_v<T, SyncExt>) {
                              t.evt->host_wait(t.value);
                          } else if constexpr (std::is_same_v<T, NotifyEvt>) {
                              t.evt->notify(t.value);
                          }
                      });
              }
              std::unique_lock lck{_mtx};
              while (_enabled && _exec.length() == 0) {
                  _cv.wait(lck);
              }
          }
      }),
      upload_alloc(TEMP_SIZE),
      default_alloc(TEMP_SIZE),
      readback_alloc(TEMP_SIZE) {
    VkCommandPoolCreateInfo pool_ci{
        .sType = VK_STRUCTURE_TYPE_COMMAND_POOL_CREATE_INFO};
    switch (tag) {
        case StreamTag::GRAPHICS:
            pool_ci.queueFamilyIndex = device->graphics_queue_index();
            _queue = device->graphics_queue();
            break;
        case StreamTag::COPY:
            pool_ci.queueFamilyIndex = device->copy_queue_index();
            _queue = device->compute_queue();
            break;
        case StreamTag::COMPUTE:
            pool_ci.queueFamilyIndex = device->compute_queue_index();
            _queue = device->copy_queue();
            break;
        default:
            LUISA_ASSERT(false, "Illegal stream tag.");
    }
    VK_CHECK_RESULT(vkCreateCommandPool(device->logic_device(), &pool_ci, Device::alloc_callbacks(), &_pool));
}
Stream::~Stream() {
    sync();
    {
        std::lock_guard lck{_mtx};
        _enabled = false;
    }
    _cv.notify_one();
    _thd.join();
    vkDestroyCommandPool(device()->logic_device(), _pool, Device::alloc_callbacks());
}
void Stream::dispatch(
    vstd::span<const luisa::unique_ptr<Command>> cmds,
    luisa::vector<luisa::move_only_function<void()>> &&callbacks,
    bool inqueue_limit) {

    if (cmds.empty() && callbacks.empty()) {
        return;
    }
    if (inqueue_limit) {
        if (_evt.last_fence() > 2) {
            _evt.sync(_evt.last_fence() - 2);
        }
    }
    auto fence = _evt.last_fence() + 1;
    if (!cmds.empty()) {
        CommandBuffer cmdbuffer{*this};
        auto cb = cmdbuffer.cmdbuffer();
        cmdbuffer.begin();
        cmdbuffer.end();
        _evt.signal(*this, fence, &cb);
        _exec.push(SyncExt{
            .evt = &_evt,
            .value = fence});
        _exec.push(std::move(cmdbuffer));
    } else {
        _evt.update_fence(fence);
    }
    if (!callbacks.empty()) {
        _exec.push(std::move(callbacks));
    }
    _exec.push(NotifyEvt{
        .evt = &_evt,
        .value = fence});

    _mtx.lock();
    _mtx.unlock();
    _cv.notify_one();
}
void Stream::sync() {
    _evt.sync(_evt.last_fence());
}
CommandBuffer::CommandBuffer(Stream &stream)
    : Resource(stream.device()),
      _pool(stream.pool()) {
    VkCommandBufferAllocateInfo cb_ci{
        .sType = VK_STRUCTURE_TYPE_COMMAND_BUFFER_ALLOCATE_INFO,
        .commandPool = _pool,
        .commandBufferCount = 1};
    VK_CHECK_RESULT(vkAllocateCommandBuffers(device()->logic_device(), &cb_ci, &_cmdbuffer));
    VkFenceCreateInfo fence_info{VK_STRUCTURE_TYPE_FENCE_CREATE_INFO};
    VK_CHECK_RESULT(vkCreateFence(device()->logic_device(), &fence_info, Device::alloc_callbacks(), nullptr));
}
CommandBuffer::~CommandBuffer() {
    if (_cmdbuffer)
        vkFreeCommandBuffers(device()->logic_device(), _pool, 1, &_cmdbuffer);
}
void CommandBuffer::begin() {
    VkCommandBufferBeginInfo bi{
        .sType = VK_STRUCTURE_TYPE_COMMAND_BUFFER_BEGIN_INFO,
    };
    VK_CHECK_RESULT(vkBeginCommandBuffer(_cmdbuffer, &bi));
}
void CommandBuffer::end() {
    VK_CHECK_RESULT(vkEndCommandBuffer(_cmdbuffer));
}
CommandBuffer::CommandBuffer(CommandBuffer &&rhs)
    : Resource(std::move(rhs)),
      _pool(rhs._pool),
      _cmdbuffer(rhs._cmdbuffer) {
    rhs._cmdbuffer = nullptr;
}
void Stream::signal(Event *event, uint64_t value) {
    event->signal(*this, value);
    _exec.push(SyncExt{event, value});
    _exec.push(NotifyEvt{event, value});
    _mtx.lock();
    _mtx.unlock();
    _cv.notify_one();
}
void Stream::wait(Event *event, uint64_t value) {
    event->wait(*this, value);
}
namespace temp_buffer {

template<typename Pack>
uint64 Visitor<Pack>::allocate(uint64 size) {
    return reinterpret_cast<uint64_t>(new Pack(device, size));
}
template<typename Pack>
void Visitor<Pack>::deallocate(uint64 handle) {
    delete reinterpret_cast<Pack *>(handle);
}
template<typename T>
void BufferAllocator<T>::clear() {
    largeBuffers.clear();
    alloc.dispose();
}
template<typename T>
BufferAllocator<T>::BufferAllocator(size_t initCapacity)
    : alloc(initCapacity, &visitor) {
}
template<typename T>
BufferAllocator<T>::~BufferAllocator() {
}
template<typename T>
BufferView BufferAllocator<T>::allocate(size_t size) {
    if (size <= kLargeBufferSize) [[likely]] {
        auto chunk = alloc.allocate(size);
        return BufferView(reinterpret_cast<T const *>(chunk.handle), chunk.offset, size);
    } else {
        auto &v = largeBuffers.emplace_back(visitor.Create(size));
        return BufferView(v.get(), 0, size);
    }
}
template<typename T>
BufferView BufferAllocator<T>::allocate(size_t size, size_t align) {
    if (size <= kLargeBufferSize) [[likely]] {
        auto chunk = alloc.allocate(size, align);
        return BufferView(reinterpret_cast<T const *>(chunk.handle), chunk.offset, size);
    } else {
        auto &v = largeBuffers.emplace_back(visitor.Create(size));
        return BufferView(v.get(), 0, size);
    }
}

}// namespace temp_buffer
}// namespace lc::vk
