#include "dml_ext.h"
#include "LCCmdBuffer.h"
#include <luisa/runtime/stream.h>
#define _D3D12MA_IUNKNOWN_IMPL_FUNCTIONS
#include "DirectMLX.h"
#include <luisa/backends/ext/dx_custom_cmd.h>
#include<wrl/client.h>
//#include <wil/result_macros.h>
#define THROW_IF_FAILED(hr) hr
#define IID_GRAPHICS_PPV_ARGS IID_PPV_ARGS
using namespace luisa;
using namespace luisa::compute;

class DxDMLGraph : public DMLGraph
{
public:
    ComPtr<IDMLDevice> dmlDevice;
    ComPtr<IDMLCompiledOperator> dmlCompiledOperator;

    ComPtr<IDMLBindingTable> dmlBindingTable;
    ComPtr<IDMLCommandRecorder> dmlCommandRecorder;
    ComPtr<ID3D12DescriptorHeap> descriptorHeap;
    size_t weightSize;
    size_t outputSize;
    size_t inputSize;
    size_t descriptorCount;
    size_t temporaryResourceSize;
    size_t persistentResourceSize;

    int layer;
    int input;
    int output;
    int hiddenDim;

    bool bind = false;
    bool half = true;


    ComPtr<ID3D12Resource> temporaryBuffer;
    ComPtr<ID3D12Resource> persistentBuffer;
};
class DxGraphBuildCommand : public DXCustomCmd
{
public:
    DxGraphBuildCommand(DxDMLGraph* graph, int batchSize, int input, int layer, int hiddenDim, int output) : 
        dmlGraph(graph), batchSize(batchSize), input(input), layer(layer), hiddenDim(hiddenDim), output(output)
    {
        graph->layer = layer;
        graph->input = input;
        graph->output = output;
        graph->hiddenDim = hiddenDim;
    }
    [[nodiscard]] virtual StreamTag stream_tag() const noexcept override
    {
        return StreamTag::COMPUTE;
    }
private:
    DxDMLGraph* dmlGraph;
    int batchSize;
    int input;
    int layer;
    int hiddenDim;
    int output;

    virtual void execute(
        IDXGIAdapter1* adapter,
        IDXGIFactory2* dxgi_factory,
        ID3D12Device* device,
        ID3D12GraphicsCommandList4* command_list) const noexcept;
};

void DxGraphBuildCommand::execute(IDXGIAdapter1* adapter, IDXGIFactory2* dxgi_factory, ID3D12Device* device, ID3D12GraphicsCommandList4* command_list) const noexcept
{
    unsigned int dataSize = dmlGraph->half ? 2 : 4;
    DML_TENSOR_DATA_TYPE dataType = dmlGraph->half ? DML_TENSOR_DATA_TYPE_FLOAT16 : DML_TENSOR_DATA_TYPE_FLOAT32;
    DML_CREATE_DEVICE_FLAGS dmlCreateDeviceFlags = DML_CREATE_DEVICE_FLAG_NONE;
    THROW_IF_FAILED(DMLCreateDevice(
        device,
        dmlCreateDeviceFlags,
        IID_PPV_ARGS(dmlGraph->dmlDevice.GetAddressOf())));

    dml::Graph graph(dmlGraph->dmlDevice.Get());
    UINT tensorSizes[4] = { 1, 1, UINT(batchSize), UINT(input) };
    dml::TensorDesc::Dimensions inputDimensions(std::begin(tensorSizes), std::end(tensorSizes));
    dml::TensorDesc desc = { dataType, inputDimensions };
    dml::Expression inputLayer = dml::InputTensor(graph, 0, desc);


    std::vector<dml::Expression> expressions{};
    int lastDim = input;
    auto& lastOutput = inputLayer;
    for (int i = 0; i < layer; i++)
    {
        UINT matrixSizes[4] = { 1, 1, UINT(hiddenDim), UINT(lastDim) };
        dml::TensorDesc::Dimensions matrixDimensions = dml::TensorDesc::Dimensions(std::begin(matrixSizes), std::end(matrixSizes));
        auto mdesc = dml::TensorDesc{ dataType, matrixDimensions };
        dml::Expression& weights = expressions.emplace_back(dml::InputTensor(graph, i + 1, mdesc));
        dml::Expression& fc = expressions.emplace_back(dml::Gemm(lastOutput, weights,
            dml::NullOpt, DML_MATRIX_TRANSFORM_NONE, DML_MATRIX_TRANSFORM_TRANSPOSE, 1.f, 1.f, dml::FusedActivation::Relu()));
        lastDim = hiddenDim;
        lastOutput = fc;
    }
    {
        UINT matrixSizes[4] = { 1, 1, UINT(output), UINT(lastDim) };
        dml::TensorDesc::Dimensions matrixDimensions = dml::TensorDesc::Dimensions(std::begin(matrixSizes), std::end(matrixSizes));
        auto mdesc = dml::TensorDesc{ dataType, matrixDimensions };
        dml::Expression& weights = expressions.emplace_back(dml::InputTensor(graph, layer + 1, mdesc));
        dml::Expression& fc = expressions.emplace_back(dml::Gemm(lastOutput, weights, dml::NullOpt, DML_MATRIX_TRANSFORM_NONE, DML_MATRIX_TRANSFORM_TRANSPOSE));
        lastDim = hiddenDim;
        lastOutput = fc;
    }
    int numWeights = input * hiddenDim + hiddenDim * hiddenDim * (layer)+hiddenDim * output;
    if (layer == 0)
    {
        numWeights = input * output;
    }
    dmlGraph->weightSize = numWeights * dataSize;
    dmlGraph->outputSize = output * batchSize * dataSize;
    dmlGraph->inputSize = input * batchSize * dataSize;

    DML_EXECUTION_FLAGS executionFlags = DML_EXECUTION_FLAG_ALLOW_HALF_PRECISION_COMPUTATION;
    dmlGraph->dmlCompiledOperator.Attach(graph.Compile(executionFlags, { lastOutput }).Detach());




    ComPtr<IDMLOperatorInitializer> dmlOperatorInitializer;
    IDMLCompiledOperator* dmlCompiledOperators[] = { dmlGraph->dmlCompiledOperator.Get() };
    THROW_IF_FAILED(dmlGraph->dmlDevice->CreateOperatorInitializer(
        ARRAYSIZE(dmlCompiledOperators),
        dmlCompiledOperators,
        IID_PPV_ARGS(dmlOperatorInitializer.GetAddressOf())));

    // Query the operator for the required size (in descriptors) of its binding table.
    // You need to initialize an operator exactly once before it can be executed, and
    // the two stages require different numbers of descriptors for binding. For simplicity,
    // we create a single descriptor heap that's large enough to satisfy them both.
    DML_BINDING_PROPERTIES initializeBindingProperties = dmlOperatorInitializer->GetBindingProperties();
    DML_BINDING_PROPERTIES executeBindingProperties = dmlGraph->dmlCompiledOperator->GetBindingProperties();
    dmlGraph->descriptorCount = std::max(
        initializeBindingProperties.RequiredDescriptorCount,
        executeBindingProperties.RequiredDescriptorCount);

    // Create descriptor heaps.

    D3D12_DESCRIPTOR_HEAP_DESC descriptorHeapDesc{};
    descriptorHeapDesc.Type = D3D12_DESCRIPTOR_HEAP_TYPE_CBV_SRV_UAV;
    descriptorHeapDesc.NumDescriptors = dmlGraph->descriptorCount;
    descriptorHeapDesc.Flags = D3D12_DESCRIPTOR_HEAP_FLAG_SHADER_VISIBLE;
    THROW_IF_FAILED(device->CreateDescriptorHeap(
        &descriptorHeapDesc,
        IID_GRAPHICS_PPV_ARGS(dmlGraph->descriptorHeap.GetAddressOf())));

    // Set the descriptor heap(s).
    ID3D12DescriptorHeap* d3D12DescriptorHeaps[] = { dmlGraph->descriptorHeap.Get() };
    command_list->SetDescriptorHeaps(ARRAYSIZE(d3D12DescriptorHeaps), d3D12DescriptorHeaps);

    // Create a binding table over the descriptor heap we just created.
    DML_BINDING_TABLE_DESC dmlBindingTableDesc{};
    dmlBindingTableDesc.Dispatchable = dmlOperatorInitializer.Get();
    dmlBindingTableDesc.CPUDescriptorHandle = dmlGraph->descriptorHeap->GetCPUDescriptorHandleForHeapStart();
    dmlBindingTableDesc.GPUDescriptorHandle = dmlGraph->descriptorHeap->GetGPUDescriptorHandleForHeapStart();
    dmlBindingTableDesc.SizeInDescriptors = dmlGraph->descriptorCount;

    ComPtr<IDMLBindingTable> initBindingTable;
    THROW_IF_FAILED(dmlGraph->dmlDevice->CreateBindingTable(
        &dmlBindingTableDesc,
        IID_PPV_ARGS(initBindingTable.GetAddressOf())));

    // Create the temporary and persistent resources that are necessary for executing an operator.

    // The temporary resource is scratch memory (used internally by DirectML), whose contents you don't need to define.
    // The persistent resource is long-lived, and you need to initialize it using the IDMLOperatorInitializer.

    dmlGraph->temporaryResourceSize = std::max(
        initializeBindingProperties.TemporaryResourceSize,
        executeBindingProperties.TemporaryResourceSize);
    dmlGraph->persistentResourceSize = executeBindingProperties.PersistentResourceSize;

    // Bind and initialize the operator on the GPU.

    auto heap = CD3DX12_HEAP_PROPERTIES(D3D12_HEAP_TYPE_DEFAULT);
    if (dmlGraph->temporaryResourceSize != 0)
    {
        auto bufferDesc = CD3DX12_RESOURCE_DESC::Buffer(dmlGraph->temporaryResourceSize, D3D12_RESOURCE_FLAG_ALLOW_UNORDERED_ACCESS);
        THROW_IF_FAILED(device->CreateCommittedResource(
            &heap,
            D3D12_HEAP_FLAG_NONE,
            &bufferDesc,
            D3D12_RESOURCE_STATE_COMMON,
            nullptr,
            IID_GRAPHICS_PPV_ARGS(dmlGraph->temporaryBuffer.GetAddressOf())));

        if (initializeBindingProperties.TemporaryResourceSize != 0)
        {
            DML_BUFFER_BINDING bufferBinding{ dmlGraph->temporaryBuffer.Get(), 0, dmlGraph->temporaryResourceSize };
            DML_BINDING_DESC bindingDesc{ DML_BINDING_TYPE_BUFFER, &bufferBinding };
            initBindingTable->BindTemporaryResource(&bindingDesc);
        }
    }

    if (dmlGraph->persistentResourceSize != 0)
    {
        auto bufferDesc = CD3DX12_RESOURCE_DESC::Buffer(dmlGraph->persistentResourceSize);
        THROW_IF_FAILED(device->CreateCommittedResource(
            &heap,
            D3D12_HEAP_FLAG_NONE,
            &bufferDesc,
            D3D12_RESOURCE_STATE_COMMON,
            nullptr,
            IID_GRAPHICS_PPV_ARGS(dmlGraph->persistentBuffer.GetAddressOf())));

        // The persistent resource should be bound as the output to the IDMLOperatorInitializer.
        DML_BUFFER_BINDING bufferBinding{ dmlGraph->persistentBuffer.Get(), 0, dmlGraph->persistentResourceSize };
        DML_BINDING_DESC bindingDesc{ DML_BINDING_TYPE_BUFFER, &bufferBinding };
        initBindingTable->BindOutputs(1, &bindingDesc);
    }

    // The command recorder is a stateless object that records Dispatches into an existing Direct3D 12 command list.
    THROW_IF_FAILED(dmlGraph->dmlDevice->CreateCommandRecorder(
        IID_PPV_ARGS(dmlGraph->dmlCommandRecorder.GetAddressOf())));


    dmlGraph->dmlCommandRecorder->RecordDispatch(
        command_list,
        dmlOperatorInitializer.Get(),
        initBindingTable.Get());

    THROW_IF_FAILED(dmlGraph->dmlDevice->CreateBindingTable(
        &dmlBindingTableDesc,
        IID_PPV_ARGS(dmlGraph->dmlBindingTable.GetAddressOf())));
}


class DxGraphForwardCommand : public DXCustomCmd
{
public:
    DxGraphForwardCommand(DxDMLGraph* graph, luisa::compute::Resource& ipt, luisa::compute::Resource& opt, luisa::compute::Resource& w) :
        dmlGraph(graph), input(ipt), output(opt), weight(w)
    {

    }
    [[nodiscard]] virtual StreamTag stream_tag() const noexcept override
    {
        return StreamTag::COMPUTE;
    }
private:
    DxDMLGraph* dmlGraph;
    luisa::compute::Resource& input;
    luisa::compute::Resource& output;
    luisa::compute::Resource& weight;

    virtual void execute(
        IDXGIAdapter1* adapter,
        IDXGIFactory2* dxgi_factory,
        ID3D12Device* device,
        ID3D12GraphicsCommandList4* command_list) const noexcept;
};

void DxGraphForwardCommand::execute(IDXGIAdapter1* adapter, IDXGIFactory2* dxgi_factory, ID3D12Device* device, ID3D12GraphicsCommandList4* command_list) const noexcept
{
    unsigned int dataSize = dmlGraph->half ? 2 : 4;
    if (!dmlGraph->bind)
    {
        dmlGraph->bind = true;
        DML_BINDING_TABLE_DESC dmlBindingTableDesc{};
        dmlBindingTableDesc.Dispatchable = dmlGraph->dmlCompiledOperator.Get();
        dmlBindingTableDesc.CPUDescriptorHandle = dmlGraph->descriptorHeap->GetCPUDescriptorHandleForHeapStart();
        dmlBindingTableDesc.GPUDescriptorHandle = dmlGraph->descriptorHeap->GetGPUDescriptorHandleForHeapStart();
        dmlBindingTableDesc.SizeInDescriptors = dmlGraph->descriptorCount;
        dmlBindingTableDesc.Dispatchable = dmlGraph->dmlCompiledOperator.Get();
        THROW_IF_FAILED(dmlGraph->dmlBindingTable->Reset(&dmlBindingTableDesc));

        if (dmlGraph->temporaryResourceSize != 0)
        {
            DML_BUFFER_BINDING bufferBinding{ dmlGraph->temporaryBuffer.Get(), 0, dmlGraph->temporaryResourceSize };
            DML_BINDING_DESC bindingDesc{ DML_BINDING_TYPE_BUFFER, &bufferBinding };
            dmlGraph->dmlBindingTable->BindTemporaryResource(&bindingDesc);
        }
        if (dmlGraph->persistentResourceSize != 0)
        {
            DML_BUFFER_BINDING bufferBinding{ dmlGraph->persistentBuffer.Get(), 0, dmlGraph->persistentResourceSize };
            DML_BINDING_DESC bindingDesc{ DML_BINDING_TYPE_BUFFER, &bufferBinding };
            dmlGraph->dmlBindingTable->BindPersistentResource(&bindingDesc);
        }
        {

            std::vector<DML_BINDING_DESC> inputBindingDescs{};
            std::vector<DML_BUFFER_BINDING> inputBufferBindings{};
            inputBufferBindings.resize(dmlGraph->layer + 2);
            inputBufferBindings[0] = DML_BUFFER_BINDING{ (ID3D12Resource*)input.native_handle(), 0, dmlGraph->inputSize };
            inputBindingDescs.emplace_back(DML_BINDING_DESC{ DML_BINDING_TYPE_BUFFER, &inputBufferBindings[0] });
            int lastDim = dmlGraph->input;
            size_t offset = 0;
            for (int i = 0; i < dmlGraph->layer; i++)
            {
                inputBufferBindings[i + 1] = DML_BUFFER_BINDING{ (ID3D12Resource*)weight.native_handle(), offset, size_t(lastDim) * dmlGraph->hiddenDim * dataSize };
                inputBindingDescs.emplace_back(DML_BINDING_DESC{ DML_BINDING_TYPE_BUFFER, &inputBufferBindings[i + 1] });
                offset += inputBufferBindings[i + 1].SizeInBytes;
                lastDim = dmlGraph->hiddenDim;
            }
            {
                inputBufferBindings[dmlGraph->layer + 1] = DML_BUFFER_BINDING{ (ID3D12Resource*)weight.native_handle(), offset, size_t(lastDim) * dmlGraph->output * dataSize };
                inputBindingDescs.emplace_back(DML_BINDING_DESC{ DML_BINDING_TYPE_BUFFER, &inputBufferBindings[dmlGraph->layer + 1] });
            }

            dmlGraph->dmlBindingTable->BindInputs(inputBindingDescs.size(), inputBindingDescs.data());
        }
        {
            DML_BUFFER_BINDING outputBufferBinding{ (ID3D12Resource*)output.native_handle(), 0, dmlGraph->outputSize };
            DML_BINDING_DESC outputBindingDesc{ DML_BINDING_TYPE_BUFFER, &outputBufferBinding };
            dmlGraph->dmlBindingTable->BindOutputs(1, &outputBindingDesc);
        }
    }

    ID3D12DescriptorHeap* d3D12DescriptorHeaps[] = { dmlGraph->descriptorHeap.Get() };
    command_list->SetDescriptorHeaps(ARRAYSIZE(d3D12DescriptorHeaps), d3D12DescriptorHeaps);
    {
        auto bt = CD3DX12_RESOURCE_BARRIER::UAV((ID3D12Resource*)input.native_handle());
        command_list->ResourceBarrier(
            1,
            &bt
        );
    }
    //Dispatch the operator
    dmlGraph->dmlCommandRecorder->RecordDispatch(command_list, dmlGraph->dmlCompiledOperator.Get(), dmlGraph->dmlBindingTable.Get());
}

lc::dx::DxDirectMLExt::DxDirectMLExt(DeviceInterface* device):device(device)
{
}

luisa::unique_ptr<DMLGraph> lc::dx::DxDirectMLExt::Build(Stream& stream, int batchSize, int input, int layer, int hiddenDim, int output, bool half)
{
    auto result = luisa::make_unique<DxDMLGraph>();
    result->half = half;
    CommandList cl{};
    cl << luisa::make_unique<DxGraphBuildCommand>(result.get(), batchSize, input, layer, hiddenDim, output);
    stream << cl.commit() << synchronize();
    return result;
}

luisa::unique_ptr<Command> lc::dx::DxDirectMLExt::Forward(DMLGraph* graph, luisa::compute::Resource& input, luisa::compute::Resource& output, luisa::compute::Resource& weights)
{
    return luisa::make_unique<DxGraphForwardCommand>((DxDMLGraph*)graph, input, output, weights);
}

