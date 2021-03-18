//
// Created by Mike Smith on 2021/3/18.
//

#include <runtime/stream.h>

#include <utility>

namespace luisa::compute {

void Stream::_dispatch(std::unique_ptr<CommandBuffer> cb) noexcept { _device->_dispatch(_handle, std::move(cb)); }

Stream::Delegate Stream::operator<<(std::unique_ptr<Command> cmd) noexcept {
    Delegate delegate{this};
    delegate << std::move(cmd);
    return delegate;
}

Stream &Stream::operator<<(std::function<void()> f) noexcept {
    return Delegate{this} << std::move(f);
}

Stream &Stream::operator<<(SynchronizeToken) {
    _device->_synchronize_stream(_handle);
    return *this;
}

Stream::Stream(Stream &&s) noexcept
    : _device{s._device}, _handle{s._handle} { s._device = nullptr; }

Stream::~Stream() noexcept {
    if (_device != nullptr) {
        _device->_dispose_stream(_handle);
    }
}

Stream &Stream::operator=(Stream &&rhs) noexcept {
    _device = rhs._device;
    _handle = rhs._handle;
    rhs._device = nullptr;
    return *this;
}

Stream::Delegate::~Delegate() noexcept { _commit(); }

Stream::Delegate &Stream::Delegate::operator<<(std::unique_ptr<Command> cmd) noexcept {
    _cb->append(std::move(cmd));
    return *this;
}

Stream &Stream::Delegate::operator<<(std::function<void()> f) noexcept {
    _cb->set_callback(std::move(f));
    _commit();
    return *_stream;
}

Stream::Delegate::Delegate(Stream *s) noexcept
    : _stream{s}, _cb{std::make_unique<CommandBuffer>()} {}

Stream &Stream::Delegate::operator<<(Stream::SynchronizeToken) noexcept {
    _commit();
    return (*_stream) << SynchronizeToken{};
}

void Stream::Delegate::_commit() noexcept {
    if (_cb != nullptr) { _stream->_dispatch(std::move(_cb)); }
}

}// namespace luisa::compute
