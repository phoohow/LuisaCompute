use serde::ser::SerializeStruct;
use serde::{Deserialize, Serialize, Serializer};

use crate::usage_detect::detect_usage;
use crate::*;
use std::any::{Any, TypeId};
use std::collections::HashSet;
use std::fmt::{Debug, Formatter};
use std::hash::Hasher;
use std::ops::Deref;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(C)]
#[derive(Serialize, Deserialize)]
pub enum Primitive {
    Bool,
    Int16,
    Uint16,
    Int32,
    Uint32,
    Int64,
    Uint64,
    Float32,
    Float64,
}

impl std::fmt::Display for Primitive {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Bool => "bool",
                Self::Int16 => "i16",
                Self::Uint16 => "u16",
                Self::Int32 => "i32",
                Self::Uint32 => "u32",
                Self::Int64 => "i64",
                Self::Uint64 => "u64",
                Self::Float32 => "f32",
                Self::Float64 => "f64",
            }
        )
    }
}

//cbindgen:derive-tagged-enum-destructor
//cbindgen:derive-tagged-enum-copy-constructor
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize)]
#[repr(C)]
pub enum VectorElementType {
    Scalar(Primitive),
    Vector(CArc<VectorType>),
}

impl VectorElementType {
    pub fn as_primitive(&self) -> Option<Primitive> {
        match self {
            VectorElementType::Scalar(p) => Some(*p),
            _ => None,
        }
    }
    pub fn is_float(&self) -> bool {
        match self {
            VectorElementType::Scalar(Primitive::Float32) => true,
            VectorElementType::Scalar(Primitive::Float64) => true,
            VectorElementType::Vector(v) => v.element.is_float(),
            _ => false,
        }
    }
    pub fn is_int(&self) -> bool {
        match self {
            VectorElementType::Scalar(Primitive::Int32) => true,
            VectorElementType::Scalar(Primitive::Uint32) => true,
            VectorElementType::Scalar(Primitive::Int64) => true,
            VectorElementType::Scalar(Primitive::Uint64) => true,
            VectorElementType::Vector(v) => v.element.is_int(),
            _ => false,
        }
    }
    pub fn is_bool(&self) -> bool {
        match self {
            VectorElementType::Scalar(Primitive::Bool) => true,
            VectorElementType::Vector(v) => v.element.is_bool(),
            _ => false,
        }
    }
    pub fn to_type(&self) -> CArc<Type> {
        match self {
            VectorElementType::Scalar(p) => context::register_type(Type::Primitive(*p)),
            VectorElementType::Vector(v) => context::register_type(Type::Vector(v.deref().clone())),
        }
    }
}

impl std::fmt::Display for VectorElementType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Scalar(primitive) => std::fmt::Display::fmt(primitive, f),
            Self::Vector(vector) => std::fmt::Display::fmt(vector, f),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize)]
#[repr(C)]
pub struct VectorType {
    pub element: VectorElementType,
    pub length: u32,
}

impl std::fmt::Display for VectorType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Vec<{};{}>", self.element, self.length)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize)]
#[repr(C)]
pub struct MatrixType {
    pub element: VectorElementType,
    pub dimension: u32,
}

impl std::fmt::Display for MatrixType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Mat<{};{}>", self.element, self.dimension)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize)]
#[repr(C)]
pub struct StructType {
    pub fields: CBoxedSlice<CArc<Type>>,
    pub alignment: usize,
    pub size: usize,
    // pub id: u64,
}

impl std::fmt::Display for StructType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Struct<")?;
        for field in self.fields.as_ref().iter() {
            write!(f, "{},", field)?;
        }
        write!(f, ">")?;
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize)]
#[repr(C)]
pub struct ArrayType {
    pub element: CArc<Type>,
    pub length: usize,
}

impl std::fmt::Display for ArrayType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Arr<{}; {}>", self.element, self.length)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize)]
#[repr(C)]
pub enum Type {
    Void,
    UserData,
    Primitive(Primitive),
    Vector(VectorType),
    Matrix(MatrixType),
    Struct(StructType),
    Array(ArrayType),
    Opaque(CBoxedSlice<u8>),
}

impl std::fmt::Display for Type {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UserData => write!(f, "userdata"),
            Self::Void => write!(f, "void"),
            Self::Primitive(primitive) => std::fmt::Display::fmt(primitive, f),
            Self::Vector(vector) => std::fmt::Display::fmt(vector, f),
            Self::Matrix(matrix) => std::fmt::Display::fmt(matrix, f),
            Self::Struct(struct_type) => std::fmt::Display::fmt(struct_type, f),
            Self::Array(arr) => std::fmt::Display::fmt(arr, f),
            Self::Opaque(name) => std::fmt::Display::fmt(&name.to_string(), f),
        }
    }
}

impl VectorElementType {
    pub fn size(&self) -> usize {
        match self {
            VectorElementType::Scalar(p) => p.size(),
            VectorElementType::Vector(v) => v.size(),
        }
    }
}

impl Primitive {
    pub fn size(&self) -> usize {
        match self {
            Primitive::Bool => 1,
            Primitive::Int16 => 2,
            Primitive::Uint16 => 2,
            Primitive::Int32 => 4,
            Primitive::Uint32 => 4,
            Primitive::Int64 => 8,
            Primitive::Uint64 => 8,
            Primitive::Float32 => 4,
            Primitive::Float64 => 8,
        }
    }
}

impl VectorType {
    pub fn size(&self) -> usize {
        let el_sz = self.element.size();
        let aligned_len = {
            let four = self.length / 4;
            let rem = self.length % 4;
            if rem <= 2 {
                four * 4 + rem
            } else {
                four * 4 + 4
            }
        };
        let len = match self.element {
            VectorElementType::Scalar(s) => match s {
                Primitive::Bool => self.length,
                _ => aligned_len,
            },
            VectorElementType::Vector(_) => self.length,
        };
        el_sz * len as usize
    }
    pub fn element(&self) -> Primitive {
        match self.element {
            VectorElementType::Scalar(s) => s,
            _ => unreachable!(),
        }
    }
}

impl MatrixType {
    pub fn size(&self) -> usize {
        let el_sz = self.element.size();
        let len = match self.element {
            VectorElementType::Scalar(s) => match s {
                Primitive::Float32 => {
                    (match self.dimension {
                        2 => 2u32,
                        3 => 4u32,
                        4 => 4u32,
                        _ => panic!("Invalid matrix dimension"),
                    }) * self.dimension
                }
                _ => panic!("Invalid matrix element type"),
            },
            VectorElementType::Vector(_) => todo!(),
        };
        el_sz * len as usize
    }
    pub fn column(&self) -> CArc<Type> {
        match &self.element {
            VectorElementType::Scalar(t) => Type::vector(t.clone(), self.dimension),
            VectorElementType::Vector(t) => Type::vector_vector(t.clone(), self.dimension),
        }
    }
    pub fn element(&self) -> Primitive {
        match self.element {
            VectorElementType::Scalar(s) => s,
            _ => unreachable!(),
        }
    }
}

impl Type {
    pub fn void() -> CArc<Type> {
        context::register_type(Type::Void)
    }
    pub fn userdata() -> CArc<Type> {
        context::register_type(Type::UserData)
    }
    pub fn extract(&self, i: usize) -> CArc<Type> {
        match self {
            Self::Void | Self::Primitive(_) | Self::UserData => unreachable!(),
            Self::Vector(_) => self.element(),
            Self::Matrix(mt) => mt.column(),
            Self::Array(arr) => arr.element.clone(),
            Self::Struct(s) => s.fields[i].clone(),
            Self::Opaque(_) => unreachable!(),
        }
    }
    pub fn size(&self) -> usize {
        match self {
            Type::Void | Type::UserData => 0,
            Type::Primitive(t) => t.size(),
            Type::Struct(t) => t.size,
            Type::Vector(t) => t.size(),
            Type::Matrix(t) => t.size(),
            Type::Array(t) => t.element.size() * t.length,
            Self::Opaque(_) => unreachable!(),
        }
    }
    pub fn element(&self) -> CArc<Type> {
        match self {
            Type::Void | Type::Primitive(_) | Type::UserData => {
                context::register_type(self.clone())
            }
            Type::Vector(vec_type) => vec_type.element.to_type(),
            Type::Matrix(mat_type) => mat_type.element.to_type(),
            Type::Struct(_) => CArc::null(),
            Type::Array(arr_type) => arr_type.element.clone(),
            Self::Opaque(_) => unreachable!(),
        }
    }
    pub fn dimension(&self) -> usize {
        match self {
            Type::Void | Type::UserData => 0,
            Type::Primitive(_) => 1,
            Type::Vector(vec_type) => vec_type.length as usize,
            Type::Matrix(mat_type) => mat_type.dimension as usize,
            Type::Struct(struct_type) => struct_type.fields.as_ref().len(),
            Type::Array(arr_type) => arr_type.length,
            Self::Opaque(_) => unreachable!(),
        }
    }
    pub fn alignment(&self) -> usize {
        match self {
            Type::Void | Type::UserData => 0,
            Type::Primitive(t) => t.size(),
            Type::Struct(t) => t.alignment,
            Type::Vector(t) => t.element.size(), // TODO
            Type::Matrix(t) => t.element.size(),
            Type::Array(t) => t.element.alignment(),
            Self::Opaque(_) => unreachable!(),
        }
    }
    pub fn vector_to_bool(from: &VectorType) -> CArc<VectorType> {
        match &from.element {
            VectorElementType::Scalar(_) => CArc::new(VectorType {
                element: VectorElementType::Scalar(Primitive::Bool),
                length: from.length,
            }),
            VectorElementType::Vector(v) => Type::vector_to_bool(v.deref()),
        }
    }
    pub fn bool(from: CArc<Type>) -> CArc<Type> {
        match from.deref() {
            Type::Primitive(_) => context::register_type(Type::Primitive(Primitive::Bool)),
            Type::Vector(vec_type) => match &vec_type.element {
                VectorElementType::Scalar(_) => Type::vector(Primitive::Bool, vec_type.length),
                VectorElementType::Vector(v) => {
                    Type::vector_vector(Type::vector_to_bool(v.deref()), vec_type.length)
                }
            },
            _ => panic!("Cannot convert to bool"),
        }
    }
    pub fn opaque(name: String) -> CArc<Type> {
        context::register_type(Type::Opaque(name.into()))
    }
    pub fn vector(element: Primitive, length: u32) -> CArc<Type> {
        context::register_type(Type::Vector(VectorType {
            element: VectorElementType::Scalar(element),
            length,
        }))
    }
    pub fn vector_vector(element: CArc<VectorType>, length: u32) -> CArc<Type> {
        context::register_type(Type::Vector(VectorType {
            element: VectorElementType::Vector(element),
            length,
        }))
    }
    pub fn matrix(element: Primitive, dimension: u32) -> CArc<Type> {
        context::register_type(Type::Matrix(MatrixType {
            element: VectorElementType::Scalar(element),
            dimension,
        }))
    }
    pub fn matrix_vector(element: CArc<VectorType>, dimension: u32) -> CArc<Type> {
        context::register_type(Type::Matrix(MatrixType {
            element: VectorElementType::Vector(element),
            dimension,
        }))
    }
    pub fn is_opaque(&self, name: &str) -> bool {
        match self {
            Type::Opaque(name_) => name_.to_string().as_str() == name,
            _ => false,
        }
    }
    pub fn is_primitive(&self) -> bool {
        match self {
            Type::Primitive(_) => true,
            _ => false,
        }
    }
    pub fn is_struct(&self) -> bool {
        match self {
            Type::Struct(_) => true,
            _ => false,
        }
    }
    pub fn is_float(&self) -> bool {
        match self {
            Type::Primitive(p) => match p {
                Primitive::Float32 | Primitive::Float64 => true,
                _ => false,
            },
            Type::Vector(v) => v.element.is_float(),
            Type::Matrix(m) => m.element.is_float(),
            _ => false,
        }
    }
    pub fn is_bool(&self) -> bool {
        match self {
            Type::Primitive(p) => match p {
                Primitive::Bool => true,
                _ => false,
            },
            Type::Vector(v) => v.element.is_bool(),
            Type::Matrix(m) => m.element.is_bool(),
            _ => false,
        }
    }
    pub fn is_int(&self) -> bool {
        match self {
            Type::Primitive(p) => match p {
                Primitive::Int32 | Primitive::Uint32 | Primitive::Int64 | Primitive::Uint64 => true,
                _ => false,
            },
            Type::Vector(v) => v.element.is_int(),
            Type::Matrix(m) => m.element.is_int(),
            _ => false,
        }
    }
    pub fn is_matrix(&self) -> bool {
        match self {
            Type::Matrix(_) => true,
            _ => false,
        }
    }
    pub fn is_array(&self) -> bool {
        match self {
            Type::Array(_) => true,
            _ => false,
        }
    }
    pub fn is_vector(&self) -> bool {
        match self {
            Type::Vector(_) => true,
            _ => false,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[repr(C)]
pub struct Node {
    pub type_: CArc<Type>,
    pub next: NodeRef,
    pub prev: NodeRef,
    pub instruction: CArc<Instruction>,
}

pub const INVALID_REF: NodeRef = NodeRef(0);

impl Node {
    pub fn new(instruction: CArc<Instruction>, type_: CArc<Type>) -> Node {
        Node {
            instruction,
            type_,
            next: INVALID_REF,
            prev: INVALID_REF,
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug, Hash, Serialize)]
#[repr(C)]
pub enum Func {
    ZeroInitializer,

    Assume,
    Unreachable(CBoxedSlice<u8>),
    Assert(CBoxedSlice<u8>),

    ThreadId,
    BlockId,
    DispatchId,
    DispatchSize,

    RequiresGradient,
    Backward,
    // marks the beginning of backward pass
    Gradient,
    GradientMarker,
    // marks a (node, gradient) tuple
    AccGrad,
    // grad (local), increment
    Detach,

    // (handle, instance_id) -> Mat4
    RayTracingInstanceTransform,
    RayTracingSetInstanceTransform,
    RayTracingSetInstanceOpacity,
    RayTracingSetInstanceVisibility,
    // (handle, Ray, mask) -> Hit
    // struct Ray alignas(16) { float origin[3], tmin; float direction[3], tmax; };
    // struct Hit alignas(16) { uint inst; uint prim; float u; float v; };
    RayTracingTraceClosest,
    // (handle, Ray, mask) -> bool
    RayTracingTraceAny,
    RayTracingQueryAll,
    // (ray, mask)-> rq
    RayTracingQueryAny, // (ray, mask)-> rq

    RayQueryWorldSpaceRay,
    // (rq) -> Ray
    RayQueryProceduralCandidateHit,
    // (rq) -> ProceduralHit
    RayQueryTriangleCandidateHit,
    // (rq) -> TriangleHit
    RayQueryCommittedHit,
    // (rq) -> CommitedHit
    RayQueryCommitTriangle,
    // (rq) -> ()
    RayQueryCommitProcedural,
    // (rq, f32) -> ()
    RayQueryTerminate,              // (rq) -> ()

    RasterDiscard,

    IndirectClearDispatchBuffer,
    IndirectEmplaceDispatchKernel,

    /// When referencing a Local in Call, it is always interpreted as a load
    /// However, there are cases you want to do this explicitly
    Load,

    Cast,
    Bitcast,

    // Binary op
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
    RotRight,
    RotLeft,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    MatCompMul,

    // Unary op
    Neg,
    Not,
    BitNot,

    All,
    Any,

    // select(p, a, b) => p ? a : b
    Select,
    Clamp,
    Lerp,
    Step,
    Saturate,

    Abs,
    Min,
    Max,

    // reduction
    ReduceSum,
    ReduceProd,
    ReduceMin,
    ReduceMax,

    // bit manipulation
    Clz,
    Ctz,
    PopCount,
    Reverse,

    IsInf,
    IsNan,

    Acos,
    Acosh,
    Asin,
    Asinh,
    Atan,
    Atan2,
    Atanh,

    Cos,
    Cosh,
    Sin,
    Sinh,
    Tan,
    Tanh,

    Exp,
    Exp2,
    Exp10,
    Log,
    Log2,
    Log10,
    Powi,
    Powf,

    Sqrt,
    Rsqrt,

    Ceil,
    Floor,
    Fract,
    Trunc,
    Round,

    Fma,
    Copysign,

    // Vector operations
    Cross,
    Dot,
    // outer_product(a, b) => a * b^T
    OuterProduct,
    Length,
    LengthSquared,
    Normalize,
    Faceforward,
    // reflect(i, n) => i - 2 * dot(n, i) * n
    Reflect,

    // Matrix operations
    Determinant,
    Transpose,
    Inverse,

    SynchronizeBlock,

    /// (buffer/smem, index, desired) -> old: stores desired, returns old.
    AtomicExchange,
    /// (buffer/smem, index, expected, desired) -> old: stores (old == expected ? desired : old), returns old.
    AtomicCompareExchange,
    /// (buffer/smem, index, val) -> old: stores (old + val), returns old.
    AtomicFetchAdd,
    /// (buffer/smem, index, val) -> old: stores (old - val), returns old.
    AtomicFetchSub,
    /// (buffer/smem, index, val) -> old: stores (old & val), returns old.
    AtomicFetchAnd,
    /// (buffer/smem, index, val) -> old: stores (old | val), returns old.
    AtomicFetchOr,
    /// (buffer/smem, index, val) -> old: stores (old ^ val), returns old.
    AtomicFetchXor,
    /// (buffer/smem, index, val) -> old: stores min(old, val), returns old.
    AtomicFetchMin,
    /// (buffer/smem, index, val) -> old: stores max(old, val), returns old.
    AtomicFetchMax,
    // memory access
    /// (buffer, index) -> value: reads the index-th element in buffer
    BufferRead,
    /// (buffer, index, value) -> void: writes value into the indeex
    BufferWrite,
    /// buffer -> uint: returns buffer size in *elements*
    BufferSize,
    /// (texture, coord) -> value
    Texture2dRead,
    /// (texture, coord, value) -> void
    Texture2dWrite,
    /// (texture, coord) -> value
    Texture3dRead,
    /// (texture, coord, value) -> void
    Texture3dWrite,
    ///(bindless_array, index: uint, uv: float2) -> float4
    BindlessTexture2dSample,
    ///(bindless_array, index: uint, uv: float2, level: float) -> float4
    BindlessTexture2dSampleLevel,
    ///(bindless_array, index: uint, uv: float2, ddx: float2, ddy: float2) -> float4
    BindlessTexture2dSampleGrad,
    ///(bindless_array, index: uint, uv: float2, ddx: float2, ddy: float2, min_mip: float) -> float4
    BindlessTexture2dSampleGradLevel,
    ///(bindless_array, index: uint, uv: float3) -> float4
    BindlessTexture3dSample,
    ///(bindless_array, index: uint, uv: float3, level: float) -> float4
    BindlessTexture3dSampleLevel,
    ///(bindless_array, index: uint, uv: float3, ddx: float3, ddy: float3) -> float4
    BindlessTexture3dSampleGrad,
    ///(bindless_array, index: uint, uv: float2, ddx: float2, ddy: float2, min_mip: float) -> float4
    BindlessTexture3dSampleGradLevel,
    ///(bindless_array, index: uint, coord: uint2) -> float4
    BindlessTexture2dRead,
    ///(bindless_array, index: uint, coord: uint3) -> float4
    BindlessTexture3dRead,
    ///(bindless_array, index: uint, coord: uint2, level: uint) -> float4
    BindlessTexture2dReadLevel,
    ///(bindless_array, index: uint, coord: uint3, level: uint) -> float4
    BindlessTexture3dReadLevel,
    ///(bindless_array, index: uint) -> uint2
    BindlessTexture2dSize,
    ///(bindless_array, index: uint) -> uint3
    BindlessTexture3dSize,
    ///(bindless_array, index: uint, level: uint) -> uint2
    BindlessTexture2dSizeLevel,
    ///(bindless_array, index: uint, level: uint) -> uint3
    BindlessTexture3dSizeLevel,
    /// (bindless_array, index: uint, element: uint) -> T
    BindlessBufferRead,
    /// (bindless_array, index: uint) -> uint: returns the size of the buffer in *elements*
    BindlessBufferSize(CArc<Type>),
    // (bindless_array, index: uint) -> u64: returns the type of the buffer
    BindlessBufferType,

    // scalar -> vector, the resulting type is stored in node
    Vec,
    // (scalar, scalar) -> vector
    Vec2,
    // (scalar, scalar, scalar) -> vector
    Vec3,
    // (scalar, scalar, scalar, scalar) -> vector
    Vec4,

    // (vector, indices,...) -> vector
    Permute,
    // (vector, scalar, index) -> vector
    InsertElement,
    // (vector, index) -> scalar
    ExtractElement,
    //(struct, index) -> value; the value can be passed to an Update instruction
    GetElementPtr,
    // (fields, ...) -> struct
    Struct,

    // (fields, ...) -> array
    Array,

    // scalar -> matrix, all elements are set to the scalar
    Mat,
    // vector x 2 -> matrix
    Mat2,
    // vector x 3 -> matrix
    Mat3,
    // vector x 4 -> matrix
    Mat4,

    Callable(CallableModuleRef),

    // ArgT -> ArgT
    CpuCustomOp(CArc<CpuCustomOp>),
}

#[derive(Clone, Debug, Serialize)]
#[repr(C)]
pub enum Const {
    Zero(CArc<Type>),
    One(CArc<Type>),
    Bool(bool),
    Int32(i32),
    Uint32(u32),
    Int64(i64),
    Uint64(u64),
    Float32(f32),
    Float64(f64),
    Generic(CBoxedSlice<u8>, CArc<Type>),
}

impl std::fmt::Display for Const {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Const::Zero(t) => write!(f, "0_{}", t),
            Const::One(t) => write!(f, "1_{}", t),
            Const::Bool(b) => write!(f, "{}", b),
            Const::Int32(i) => write!(f, "{}", i),
            Const::Uint32(u) => write!(f, "{}", u),
            Const::Int64(i) => write!(f, "{}", i),
            Const::Uint64(u) => write!(f, "{}", u),
            Const::Float32(fl) => write!(f, "{}", fl),
            Const::Float64(fl) => write!(f, "{}", fl),
            Const::Generic(data, t) => write!(f, "byte<{}>[{}]", t, data.as_ref().len()),
        }
    }
}

impl Const {
    pub fn get_i32(&self) -> i32 {
        match self {
            Const::Int32(v) => *v,
            Const::Uint32(v) => *v as i32,
            Const::One(t) => {
                assert!(t.is_primitive() && t.is_int(), "cannot convert {:?} to i32", t);
                1
            }
            Const::Zero(t) => {
                assert!(t.is_primitive() && t.is_int(), "cannot convert {:?} to i32", t);
                0
            }
            Const::Generic(slice, t) => {
                assert!(t.is_primitive() && t.is_int(), "cannot convert {:?} to i32", t);
                assert_eq!(slice.len(), 4, "invalid slice length for i32");
                let mut buf = [0u8; 4];
                buf.copy_from_slice(slice);
                i32::from_le_bytes(buf)
            }
            _ => panic!("cannot convert to i32"),
        }
    }
    pub fn type_(&self) -> CArc<Type> {
        match self {
            Const::Zero(ty) => ty.clone(),
            Const::One(ty) => ty.clone(),
            Const::Bool(_) => <bool as TypeOf>::type_(),
            Const::Int32(_) => <i32 as TypeOf>::type_(),
            Const::Uint32(_) => <u32 as TypeOf>::type_(),
            Const::Int64(_) => <i64 as TypeOf>::type_(),
            Const::Uint64(_) => <u64 as TypeOf>::type_(),
            Const::Float32(_) => <f32 as TypeOf>::type_(),
            Const::Float64(_) => <f64 as TypeOf>::type_(),
            Const::Generic(_, t) => t.clone(),
        }
    }
}

/// cbindgen:derive-eq
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, PartialOrd, Ord)]
#[repr(C)]
pub struct NodeRef(pub usize);

#[repr(C)]
#[derive(Debug)]
pub struct UserData {
    tag: u64,
    data: *const u8,
    eq: extern "C" fn(*const u8, *const u8) -> bool,
}

impl PartialEq for UserData {
    fn eq(&self, other: &Self) -> bool {
        (self.eq)(self.data, other.data)
    }
}

impl Eq for UserData {}

impl Serialize for UserData {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let state = serializer.serialize_struct("UserData", 1)?;
        state.end()
    }
}

#[derive(Clone, Copy, Debug, Serialize)]
#[repr(C)]
pub struct PhiIncoming {
    pub value: NodeRef,
    pub block: Pooled<BasicBlock>,
}

#[repr(C)]
#[derive(PartialEq, Eq, Hash)]
pub struct CpuCustomOp {
    pub data: *mut u8,
    /// func(data, args); func should modify args in place
    pub func: extern "C" fn(*mut u8, *mut u8),
    pub destructor: extern "C" fn(*mut u8),
    pub arg_type: CArc<Type>,
}

impl Serialize for CpuCustomOp {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let state = serializer.serialize_struct("CpuCustomOp", 1)?;
        state.end()
    }
}

impl Debug for CpuCustomOp {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        f.debug_struct("CpuCustomOp").finish()
    }
}

#[repr(C)]
#[derive(Clone, Debug, Serialize)]
pub struct SwitchCase {
    pub value: i32,
    pub block: Pooled<BasicBlock>,
}

#[repr(C)]
#[derive(Clone, Debug, Serialize)]
pub enum Instruction {
    Buffer,
    Bindless,
    Texture2D,
    Texture3D,
    Accel,
    // Shared memory
    Shared,
    // Uniform kernel arguments
    Uniform,
    Local {
        init: NodeRef,
    },
    // Callable arguments
    Argument {
        by_value: bool,
    },
    UserData(CArc<UserData>),
    Invalid,
    Const(Const),

    // a variable that can be assigned to
    // similar to LLVM's alloca
    Update {
        var: NodeRef,
        value: NodeRef,
    },

    Call(Func, CBoxedSlice<NodeRef>),

    Phi(CBoxedSlice<PhiIncoming>),
    /* represent a loop in the form of
    loop {
        body();
        if (!cond) {
            break;
        }
    }
    */
    Return(NodeRef),
    Loop {
        body: Pooled<BasicBlock>,
        cond: NodeRef,
    },
    /* represent a loop in the form of
    loop {
        prepare;// typically the computation of the loop condition
        if cond {
            body;
            update; // continue goes here
        }
    }
    for (;; update) {
        prepare;
        if (!cond) {
            break;
        }
        body;
    }
    */
    GenericLoop {
        prepare: Pooled<BasicBlock>,
        cond: NodeRef,
        body: Pooled<BasicBlock>,
        update: Pooled<BasicBlock>,
    },
    Break,
    Continue,
    If {
        cond: NodeRef,
        true_branch: Pooled<BasicBlock>,
        false_branch: Pooled<BasicBlock>,
    },
    Switch {
        value: NodeRef,
        default: Pooled<BasicBlock>,
        cases: CBoxedSlice<SwitchCase>,
    },
    AdScope {
        body: Pooled<BasicBlock>,
    },
    RayQuery {
        ray_query: NodeRef,
        on_triangle_hit: Pooled<BasicBlock>,
        on_procedural_hit: Pooled<BasicBlock>,
    },
    AdDetach(Pooled<BasicBlock>),
    Comment(CBoxedSlice<u8>),
}

extern "C" fn eq_impl<T: UserNodeData>(a: *const u8, b: *const u8) -> bool {
    let a = unsafe { &*(a as *const T) };
    let b = unsafe { &*(b as *const T) };
    a.equal(b)
}

fn type_id_u64<T: UserNodeData>() -> u64 {
    unsafe { std::mem::transmute(TypeId::of::<T>()) }
}

pub fn new_user_node<T: UserNodeData>(pools: &CArc<ModulePools>, data: T) -> NodeRef {
    new_node(
        pools,
        Node::new(
            CArc::new(Instruction::UserData(CArc::new(UserData {
                tag: type_id_u64::<T>(),
                data: Box::into_raw(Box::new(data)) as *mut u8,
                eq: eq_impl::<T>,
            }))),
            Type::userdata(),
        ),
    )
}

impl Instruction {
    pub fn is_call(&self) -> bool {
        match self {
            Instruction::Call(_, _) => true,
            _ => false,
        }
    }
    pub fn is_const(&self) -> bool {
        match self {
            Instruction::Const(_) => true,
            _ => false,
        }
    }
    pub fn is_phi(&self) -> bool {
        match self {
            Instruction::Phi(_) => true,
            _ => false,
        }
    }
    pub fn has_value(&self) -> bool {
        self.is_call() || self.is_const() || self.is_phi()
    }
}

pub const INVALID_INST: Instruction = Instruction::Invalid;

pub fn new_node(pools: &CArc<ModulePools>, node: Node) -> NodeRef {
    let ptr = pools.node_pool.alloc(node);
    NodeRef(ptr.ptr as usize)
}

pub trait UserNodeData: Any + Debug {
    fn equal(&self, other: &dyn UserNodeData) -> bool;
    fn as_any(&self) -> &dyn Any;
}
macro_rules! impl_userdata {
    ($t:ty) => {
        impl UserNodeData for $t {
            fn equal(&self, other: &dyn UserNodeData) -> bool {
                let other = other.as_any().downcast_ref::<$t>().unwrap();
                self == other
            }
            fn as_any(&self) -> &dyn Any {
                self
            }
        }
    };
}
impl_userdata!(usize);
impl_userdata!(u32);
impl_userdata!(u64);
impl_userdata!(i32);
impl_userdata!(i64);
impl_userdata!(bool);

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct BasicBlock {
    pub(crate) first: NodeRef,
    pub(crate) last: NodeRef,
}

#[derive(Serialize)]
struct NodeRefAndNode<'a> {
    id: NodeRef,
    data: &'a Node,
}

impl Serialize for BasicBlock {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut state = serializer.serialize_struct("BasicBlock", 1)?;
        let nodes = self.nodes();
        let nodes = nodes
            .iter()
            .map(|n| NodeRefAndNode {
                id: *n,
                data: n.get(),
            })
            .collect::<Vec<_>>();
        state.serialize_field("nodes", &nodes)?;
        state.end()
    }
}

pub struct BasicBlockIter<'a> {
    cur: NodeRef,
    last: NodeRef,
    _block: &'a BasicBlock,
}

impl Iterator for BasicBlockIter<'_> {
    type Item = NodeRef;
    fn next(&mut self) -> Option<Self::Item> {
        if self.cur == self.last {
            None
        } else {
            let ret = self.cur;
            self.cur = self.cur.get().next;
            Some(ret)
        }
    }
}

impl BasicBlock {
    pub fn iter(&self) -> BasicBlockIter {
        BasicBlockIter {
            cur: self.first.get().next,
            last: self.last,
            _block: self,
        }
    }
    pub fn phis(&self) -> Vec<NodeRef> {
        self.iter().filter(|n| n.is_phi()).collect()
    }
    pub fn nodes(&self) -> Vec<NodeRef> {
        self.iter().collect()
    }
    pub fn into_vec(&self) -> Vec<NodeRef> {
        let mut vec = Vec::new();
        let mut cur = self.first.get().next;
        while cur != self.last {
            vec.push(cur.clone());
            let next = cur.get().next;
            cur.update(|node| {
                node.prev = INVALID_REF;
                node.next = INVALID_REF;
            });
            cur = next;
        }
        self.first.update(|node| node.next = self.last);
        self.last.update(|node| node.prev = self.first);
        for i in &vec {
            debug_assert!(!i.is_linked());
        }
        vec
    }
    pub fn new(pools: &CArc<ModulePools>) -> Self {
        let first = new_node(
            pools,
            Node::new(CArc::new(Instruction::Invalid), Type::void()),
        );
        let last = new_node(
            pools,
            Node::new(CArc::new(Instruction::Invalid), Type::void()),
        );
        first.update(|node| node.next = last);
        last.update(|node| node.prev = first);
        Self { first, last }
    }
    pub fn push(&self, node: NodeRef) {
        // node.insert_before(self.last);
        self.last.insert_before_self(node);
    }

    pub fn is_empty(&self) -> bool {
        !self.first.valid()
    }
    pub fn len(&self) -> usize {
        let mut len = 0;
        let mut cur = self.first.get().next;
        while cur != self.last {
            len += 1;
            cur = cur.get().next;
        }
        len
    }
    pub fn merge(&self, other: Pooled<BasicBlock>) {
        let nodes = other.into_vec();
        for node in nodes {
            self.push(node);
        }
    }
    /// split the block into two at @at
    /// @at is not transfered into the other block
    pub fn split(&self, at: NodeRef, pools: &CArc<ModulePools>) -> Pooled<BasicBlock> {
        let new_bb_start = at.get().next;
        let second_last = self.last.get().prev;
        let new_bb = pools.bb_pool.alloc(BasicBlock::new(pools));
        new_bb.first.update(|first| {
            first.next = new_bb_start;
        });
        new_bb.last.update(|last| {
            last.prev = second_last;
        });
        second_last.update(|node| {
            node.next = new_bb.last;
        });
        new_bb_start.update(|node| {
            node.prev = new_bb.first;
        });
        at.update(|at| {
            at.next = self.last;
        });
        self.last.update(|last| {
            last.prev = at;
        });
        new_bb
    }
}

impl NodeRef {
    pub fn get_i32(&self) -> i32 {
        match self.get().instruction.as_ref() {
            Instruction::Const(c) => c.get_i32(),
            _ => panic!("not i32 node; found: {:?}", self.get().instruction),
        }
    }
    pub fn is_unreachable(&self) -> bool {
        match self.get().instruction.as_ref() {
            Instruction::Call(Func::Unreachable(_), _) => true,
            _ => false,
        }
    }
    pub fn get_user_data(&self) -> &UserData {
        match self.get().instruction.as_ref() {
            Instruction::UserData(data) => data,
            _ => panic!("not user data node; found: {:?}", self.get().instruction),
        }
    }
    pub fn is_user_data(&self) -> bool {
        match self.get().instruction.as_ref() {
            Instruction::UserData(_) => true,
            _ => false,
        }
    }
    pub fn unwrap_user_data<T: UserNodeData>(&self) -> &T {
        let data = self.get_user_data();
        assert_eq!(data.tag, type_id_u64::<T>());
        let data = data.data as *const T;
        unsafe { &*data }
    }
    pub fn is_local(&self) -> bool {
        match self.get().instruction.as_ref() {
            Instruction::Local { .. } => true,
            _ => false,
        }
    }
    pub fn is_const(&self) -> bool {
        match self.get().instruction.as_ref() {
            Instruction::Const(_) => true,
            _ => false,
        }
    }
    pub fn is_uniform(&self) -> bool {
        match self.get().instruction.as_ref() {
            Instruction::Uniform => true,
            _ => false,
        }
    }
    pub fn is_argument(&self) -> bool {
        match self.get().instruction.as_ref() {
            Instruction::Argument { .. } => true,
            _ => false,
        }
    }
    pub fn is_value_argument(&self) -> bool {
        match self.get().instruction.as_ref() {
            Instruction::Argument { by_value } => *by_value,
            _ => false,
        }
    }
    pub fn is_refernece_argument(&self) -> bool {
        match self.get().instruction.as_ref() {
            Instruction::Argument { by_value } => !*by_value,
            _ => false,
        }
    }
    pub fn is_phi(&self) -> bool {
        self.get().instruction.is_phi()
    }
    pub fn get<'a>(&'a self) -> &'a Node {
        assert!(self.valid());
        unsafe { &*(self.0 as *const Node) }
    }
    pub fn get_mut(&self) -> &mut Node {
        assert!(self.valid());
        unsafe { &mut *(self.0 as *mut Node) }
    }
    pub fn valid(&self) -> bool {
        self.0 != INVALID_REF.0
    }
    pub fn set(&self, node: Node) {
        *self.get_mut() = node;
    }
    pub fn update<T>(&self, f: impl FnOnce(&mut Node) -> T) -> T {
        f(self.get_mut())
    }
    pub fn access_chain(&self) -> Option<(NodeRef, Vec<(CArc<Type>, usize)>)> {
        match self.get().instruction.as_ref() {
            Instruction::Call(f, args) => {
                if *f == Func::GetElementPtr {
                    let var = args[0];
                    let idx = args[1].get_i32() as usize;
                    if let Some((parent, mut indices)) = var.access_chain() {
                        indices.push((self.type_().clone(), idx));
                        return Some((parent, indices));
                    }
                    Some((var, vec![(self.type_().clone(), idx)]))
                } else {
                    None
                }
            }
            _ => None,
        }
    }
    pub fn type_(&self) -> &CArc<Type> {
        &self.get().type_
    }
    pub fn is_linked(&self) -> bool {
        assert!(self.valid());
        self.get().prev.valid() || self.get().next.valid()
    }
    pub fn remove(&self) {
        assert!(self.valid());
        let prev = self.get().prev;
        let next = self.get().next;
        prev.update(|node| node.next = next);
        next.update(|node| node.prev = prev);
        self.update(|node| {
            node.prev = INVALID_REF;
            node.next = INVALID_REF;
        });
    }
    pub fn insert_after_self(&self, node_ref: NodeRef) {
        assert!(self.valid());
        assert!(!node_ref.is_linked());
        let next = self.get().next;
        self.update(|node| node.next = node_ref);
        next.update(|node| node.prev = node_ref);
        node_ref.update(|node| {
            node.prev = *self;
            node.next = next;
        });
    }
    pub fn insert_before_self(&self, node_ref: NodeRef) {
        assert!(self.valid());
        assert!(!node_ref.is_linked());
        let prev = self.get().prev;
        self.update(|node| node.prev = node_ref);
        prev.update(|node| node.next = node_ref);
        node_ref.update(|node| {
            node.prev = prev;
            node.next = *self;
        });
    }
    pub fn is_lvalue(&self) -> bool {
        match self.get().instruction.as_ref() {
            Instruction::Local { .. } => true,
            Instruction::Argument { by_value, .. } => !by_value,
            Instruction::Call(f, _) => *f == Func::GetElementPtr,
            _ => false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
#[repr(C)]
pub enum ModuleKind {
    Block,
    Function,
    Kernel,
}

#[repr(C)]
#[derive(Debug, Serialize)]
pub struct Module {
    pub kind: ModuleKind,
    pub entry: Pooled<BasicBlock>,
    #[serde(skip)]
    pub pools: CArc<ModulePools>,
}

#[repr(C)]
#[derive(Debug, Serialize, Clone)]
pub struct CallableModuleRef(pub CArc<CallableModule>);

#[repr(C)]
#[derive(Debug, Serialize)]
pub struct CallableModule {
    pub module: Module,
    pub ret_type: CArc<Type>,
    pub args: CBoxedSlice<NodeRef>,
    pub captures: CBoxedSlice<Capture>,
    pub callables: CBoxedSlice<CallableModuleRef>,
    pub cpu_custom_ops: CBoxedSlice<CArc<CpuCustomOp>>,
    #[serde(skip)]
    pub pools: CArc<ModulePools>,
}

impl PartialEq for CallableModuleRef {
    fn eq(&self, other: &Self) -> bool {
        self.0.as_ptr() == other.0.as_ptr()
    }
}

impl Eq for CallableModuleRef {}

impl Hash for CallableModuleRef {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.as_ptr().hash(state);
    }
}

// buffer binding
#[repr(C)]
#[derive(Debug, Serialize, Deserialize, Copy, Clone, Hash, PartialEq, Eq)]
pub struct BufferBinding {
    pub handle: u64,
    pub offset: u64,
    pub size: usize,
}

// texture binding
#[repr(C)]
#[derive(Debug, Serialize, Deserialize, Copy, Clone, Hash, PartialEq, Eq)]
pub struct TextureBinding {
    pub handle: u64,
    pub level: u32,
}

// bindless array binding
#[repr(C)]
#[derive(Debug, Serialize, Deserialize, Copy, Clone, Hash, PartialEq, Eq)]
pub struct BindlessArrayBinding {
    pub handle: u64,
}

// accel binding
#[repr(C)]
#[derive(Debug, Serialize, Deserialize, Copy, Clone, Hash, PartialEq, Eq)]
pub struct AccelBinding {
    pub handle: u64,
}

#[repr(C)]
#[derive(Debug, Serialize, Copy, Clone, Deserialize, Hash, PartialEq, Eq)]
pub enum Binding {
    Buffer(BufferBinding),
    Texture(TextureBinding),
    BindlessArray(BindlessArrayBinding),
    Accel(AccelBinding),
}

#[derive(Debug, Serialize, Copy, Clone, Hash, PartialEq, Eq)]
#[repr(C)]
pub struct Capture {
    pub node: NodeRef,
    pub binding: Binding,
}

#[derive(Debug)]
pub struct ModulePools {
    pub node_pool: Pool<Node>,
    pub bb_pool: Pool<BasicBlock>,
}

impl ModulePools {
    pub fn new() -> Self {
        Self {
            node_pool: Pool::new(),
            bb_pool: Pool::new(),
        }
    }
}

#[repr(C)]
#[derive(Debug, Serialize)]
pub struct KernelModule {
    pub module: Module,
    pub captures: CBoxedSlice<Capture>,
    pub args: CBoxedSlice<NodeRef>,
    pub shared: CBoxedSlice<NodeRef>,
    pub cpu_custom_ops: CBoxedSlice<CArc<CpuCustomOp>>,
    pub callables: CBoxedSlice<CallableModuleRef>,
    pub block_size: [u32; 3],
    #[serde(skip)]
    pub pools: CArc<ModulePools>,
}

unsafe impl Send for KernelModule {}

#[repr(C)]
#[derive(Debug, Serialize)]
pub struct BlockModule {
    pub module: Module,
}

unsafe impl Send for BlockModule {}

impl Module {
    pub fn from_fragment(entry: Pooled<BasicBlock>, pools: CArc<ModulePools>) -> Self {
        Self {
            kind: ModuleKind::Block,
            entry,
            pools,
        }
    }
}

struct NodeCollector {
    nodes: Vec<NodeRef>,
    unique: HashSet<NodeRef>,
}

impl NodeCollector {
    fn new() -> Self {
        Self {
            nodes: Vec::new(),
            unique: HashSet::new(),
        }
    }
    fn visit_block(&mut self, block: Pooled<BasicBlock>) {
        for node in block.iter() {
            self.visit_node(node);
        }
    }
    fn visit_node(&mut self, node_ref: NodeRef) {
        if self.unique.contains(&node_ref) {
            return;
        }
        self.unique.insert(node_ref);
        self.nodes.push(node_ref);
        let inst = node_ref.get().instruction.as_ref();
        match inst {
            Instruction::AdScope { body } => {
                self.visit_block(*body);
            }
            Instruction::If {
                cond: _,
                true_branch,
                false_branch,
            } => {
                self.visit_block(*true_branch);
                self.visit_block(*false_branch);
            }
            Instruction::Loop { body, cond: _ } => {
                self.visit_block(*body);
            }
            Instruction::GenericLoop {
                prepare,
                cond: _,
                body,
                update,
            } => {
                self.visit_block(*prepare);
                self.visit_block(*body);
                self.visit_block(*update);
            }
            Instruction::Switch {
                value: _,
                default,
                cases,
            } => {
                self.visit_block(*default);
                for SwitchCase { value: _, block } in cases.as_ref().iter() {
                    self.visit_block(*block);
                }
            }
            _ => {}
        }
    }
}

impl Module {
    pub fn collect_nodes(&self) -> Vec<NodeRef> {
        let mut collector = NodeCollector::new();
        collector.visit_block(self.entry);
        collector.nodes
    }
}
// struct ModuleCloner {
//     node_map: HashMap<NodeRef, NodeRef>,
// }

// impl ModuleCloner {
//     fn new() -> Self {
//         Self {
//             node_map: HashMap::new(),
//         }
//     }
//     fn clone_node(&mut self, node: NodeRef, builder: &mut IrBuilder) -> NodeRef {
//         if let Some(&node) = self.node_map.get(&node) {
//             return node;
//         }
//         let new_node = match node.get().instruction.as_ref() {
//             Instruction::Buffer => node,
//             Instruction::Bindless => node,
//             Instruction::Texture2D => node,
//             Instruction::Texture3D => node,
//             Instruction::Accel => node,
//             Instruction::Shared => node,
//             Instruction::Uniform => node,
//             Instruction::Local { .. } => todo!(),
//             Instruction::Argument { .. } => todo!(),
//             Instruction::UserData(_) => node,
//             Instruction::Invalid => node,
//             Instruction::Const(_) => todo!(),
//             Instruction::Update { var, value } => todo!(),
//             Instruction::Call(_, _) => todo!(),
//             Instruction::Phi(_) => todo!(),
//             Instruction::Loop { body, cond } => todo!(),
//             Instruction::GenericLoop { .. } => todo!(),
//             Instruction::Break => builder.break_(),
//             Instruction::Continue => builder.continue_(),
//             Instruction::Return(_) => todo!(),
//             Instruction::If { .. } => todo!(),
//             Instruction::Switch { .. } => todo!(),
//             Instruction::AdScope { .. } => todo!(),
//             Instruction::AdDetach(_) => todo!(),
//             Instruction::Comment(_) => builder.clone_node(node),
//             crate::ir::Instruction::Debug { .. } => builder.clone_node(node),
//         };
//         self.node_map.insert(node, new_node);
//         new_node
//     }
//     fn clone_block(
//         &mut self,
//         block: Pooled<BasicBlock>,
//         mut builder: IrBuilder,
//     ) -> Pooled<BasicBlock> {
//         let mut cur = block.first.get().next;
//         while cur != block.last {
//             let _ = self.clone_node(cur, &mut builder);
//             cur = cur.get().next;
//         }
//         builder.finish()
//     }
//     fn clone_module(&mut self, module: &Module) -> Module {
//         Module {
//             kind: module.kind,
//             entry: self.clone_block(module.entry, IrBuilder::new()),
//         }
//     }
// }

// impl Clone for Module {
//     fn clone(&self) -> Self {
//         Self {
//             kind: self.kind,
//             entry: self.entry,
//         }
//     }
// }

#[repr(C)]
pub struct IrBuilder {
    bb: Pooled<BasicBlock>,
    pub(crate) pools: CArc<ModulePools>,
    pub(crate) insert_point: NodeRef,
}

impl IrBuilder {
    pub fn pools(&self) -> &CArc<ModulePools> {
        &self.pools
    }
    pub fn new(pools: CArc<ModulePools>) -> Self {
        let bb = pools.bb_pool.alloc(BasicBlock::new(&pools));
        let insert_point = bb.first;
        Self {
            bb,
            insert_point,
            pools,
        }
    }
    pub fn set_insert_point(&mut self, node: NodeRef) {
        self.insert_point = node;
    }
    pub fn append(&mut self, node: NodeRef) {
        self.insert_point.insert_after_self(node);
        self.insert_point = node;
    }
    pub fn append_block(&mut self, block: Pooled<BasicBlock>) {
        self.bb.merge(block);
        self.insert_point = self.bb.last.get().prev;
    }
    pub fn break_(&mut self) -> NodeRef {
        let new_node = new_node(
            &self.pools,
            Node::new(CArc::new(Instruction::Break), Type::void()),
        );
        self.append(new_node);
        new_node
    }
    pub fn continue_(&mut self) -> NodeRef {
        let new_node = new_node(
            &self.pools,
            Node::new(CArc::new(Instruction::Continue), Type::void()),
        );
        self.append(new_node);
        new_node
    }
    pub fn return_(&mut self, node: NodeRef) {
        let new_node = new_node(
            &self.pools,
            Node::new(CArc::new(Instruction::Return(node)), Type::void()),
        );
        self.append(new_node);
    }
    pub fn zero_initializer(&mut self, ty: CArc<Type>) -> NodeRef {
        self.call(Func::ZeroInitializer, &[], ty)
    }
    pub fn requires_gradient(&mut self, node: NodeRef) -> NodeRef {
        self.call(Func::RequiresGradient, &[node], Type::void())
    }
    pub fn gradient(&mut self, node: NodeRef) -> NodeRef {
        self.call(Func::Gradient, &[node], node.type_().clone())
    }
    pub fn clone_node(&mut self, node: NodeRef) -> NodeRef {
        let node = node.get();
        let new_node = new_node(
            &self.pools,
            Node::new(node.instruction.clone(), node.type_.clone()),
        );
        self.append(new_node);
        new_node
    }
    pub fn const_(&mut self, const_: Const) -> NodeRef {
        let t = const_.type_();
        let node = Node::new(CArc::new(Instruction::Const(const_)), t);
        let node = new_node(&self.pools, node);
        self.append(node.clone());
        node
    }
    pub fn local_zero_init(&mut self, ty: CArc<Type>) -> NodeRef {
        let node = self.zero_initializer(ty);
        let local = self.local(node);
        local
    }
    pub fn local(&mut self, init: NodeRef) -> NodeRef {
        let t = init.type_();
        let node = Node::new(CArc::new(Instruction::Local { init }), t.clone());
        let node = new_node(&self.pools, node);
        self.append(node.clone());
        node
    }
    pub fn store(&mut self, var: NodeRef, value: NodeRef) {
        assert!(var.is_lvalue());
        let node = Node::new(CArc::new(Instruction::Update { var, value }), Type::void());
        let node = new_node(&self.pools, node);
        self.append(node);
    }
    pub fn extract(&mut self, node: NodeRef, index: usize, ret_type: CArc<Type>) -> NodeRef {
        let c = self.const_(Const::Int32(index as i32));
        self.call(Func::ExtractElement, &[node, c], ret_type)
    }
    pub fn call(&mut self, func: Func, args: &[NodeRef], ret_type: CArc<Type>) -> NodeRef {
        let node = Node::new(
            CArc::new(Instruction::Call(func, CBoxedSlice::new(args.to_vec()))),
            ret_type,
        );
        let node = new_node(&self.pools, node);
        self.append(node.clone());
        node
    }
    pub fn cast(&mut self, node: NodeRef, t: CArc<Type>) -> NodeRef {
        self.call(Func::Cast, &[node], t)
    }
    pub fn bitcast(&mut self, node: NodeRef, t: CArc<Type>) -> NodeRef {
        self.call(Func::Bitcast, &[node], t)
    }
    pub fn update(&mut self, var: NodeRef, value: NodeRef) {
        match var.get().instruction.as_ref() {
            Instruction::Local { .. } => {}
            Instruction::Call(func, _) => match func {
                Func::GetElementPtr => {}
                _ => panic!("not local or getelementptr"),
            },
            _ => panic!("not a var"),
        }
        let node = Node::new(CArc::new(Instruction::Update { var, value }), Type::void());
        let node = new_node(&self.pools, node);
        self.append(node);
    }
    pub fn phi(&mut self, incoming: &[PhiIncoming], t: CArc<Type>) -> NodeRef {
        if t == Type::userdata() {
            let userdata0 = incoming[0].value.get_user_data();
            for i in 1..incoming.len() {
                if incoming[i].value.is_unreachable() {
                    continue;
                }
                let userdata = incoming[i].value.get_user_data();
                assert_eq!(
                    userdata0.tag, userdata.tag,
                    "Different UserData node found!"
                );
                assert_eq!(userdata0.eq, userdata.eq, "Different UserData node found!");
                assert!(
                    (userdata0.eq)(userdata0.data, userdata.data),
                    "Different UserData node found!"
                );
            }
            return incoming[0].value;
        }
        let node = Node::new(
            CArc::new(Instruction::Phi(CBoxedSlice::new(incoming.to_vec()))),
            t,
        );
        let node = new_node(&self.pools, node);
        self.append(node.clone());
        node
    }
    pub fn switch(&mut self, value: NodeRef, cases: &[SwitchCase], default: Pooled<BasicBlock>) {
        let node = Node::new(
            CArc::new(Instruction::Switch {
                value,
                default,
                cases: CBoxedSlice::new(cases.to_vec()),
            }),
            Type::void(),
        );
        let node = new_node(&self.pools, node);
        self.append(node);
    }
    pub fn if_(
        &mut self,
        cond: NodeRef,
        true_branch: Pooled<BasicBlock>,
        false_branch: Pooled<BasicBlock>,
    ) -> NodeRef {
        let node = Node::new(
            CArc::new(Instruction::If {
                cond,
                true_branch,
                false_branch,
            }),
            Type::void(),
        );
        let node = new_node(&self.pools, node);
        self.append(node);
        node
    }
    pub fn ray_query(
        &mut self,
        ray_query: NodeRef,
        on_triangle_hit: Pooled<BasicBlock>,
        on_procedural_hit: Pooled<BasicBlock>,
        type_: CArc<Type>,
    ) -> NodeRef {
        let node = Node::new(
            CArc::new(Instruction::RayQuery {
                ray_query,
                on_triangle_hit,
                on_procedural_hit,
            }),
            type_,
        );
        let node = new_node(&self.pools, node);
        self.append(node);
        node
    }
    pub fn loop_(&mut self, body: Pooled<BasicBlock>, cond: NodeRef) -> NodeRef {
        let node = Node::new(CArc::new(Instruction::Loop { body, cond }), Type::void());
        let node = new_node(&self.pools, node);
        self.append(node);
        node
    }
    pub fn generic_loop(
        &mut self,
        prepare: Pooled<BasicBlock>,
        cond: NodeRef,
        body: Pooled<BasicBlock>,
        update: Pooled<BasicBlock>,
    ) -> NodeRef {
        let node = Node::new(
            CArc::new(Instruction::GenericLoop {
                prepare,
                cond,
                body,
                update,
            }),
            Type::void(),
        );
        let node = new_node(&self.pools, node);
        self.append(node);
        node
    }
    pub fn finish(self) -> Pooled<BasicBlock> {
        self.bb
    }
}

#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(C)]
pub enum Usage {
    NONE,
    READ,
    WRITE,
    READ_WRITE,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(C)]
pub enum UsageMark {
    READ,
    WRITE,
}

impl Usage {
    pub fn mark(&self, usage: UsageMark) -> Usage {
        match (self, usage) {
            (Usage::NONE, UsageMark::READ) => Usage::READ,
            (Usage::NONE, UsageMark::WRITE) => Usage::WRITE,
            (Usage::READ, UsageMark::READ) => Usage::READ,
            (Usage::READ, UsageMark::WRITE) => Usage::READ_WRITE,
            (Usage::WRITE, UsageMark::READ) => Usage::READ_WRITE,
            (Usage::WRITE, UsageMark::WRITE) => Usage::WRITE,
            (Usage::READ_WRITE, _) => Usage::READ_WRITE,
        }
    }
    pub fn to_u8(&self) -> u8 {
        match self {
            Usage::NONE => 0,
            Usage::READ => 1,
            Usage::WRITE => 2,
            Usage::READ_WRITE => 3,
        }
    }
}

#[no_mangle]
pub extern "C" fn luisa_compute_ir_node_usage(kernel: &KernelModule) -> CBoxedSlice<u8> {
    let mut usage_map = detect_usage(&kernel.module);
    let mut usage = Vec::new();
    for captured in kernel.captures.as_ref() {
        usage.push(
            usage_map
                .remove(&captured.node)
                .expect(
                    format!(
                        "Requested resource {} not exist in usage map",
                        captured.node.0
                    )
                        .as_str(),
                )
                .to_u8(),
        );
    }
    for argument in kernel.args.as_ref() {
        usage.push(
            usage_map
                .remove(argument)
                .expect(
                    format!("Requested argument {} not exist in usage map", argument.0).as_str(),
                )
                .to_u8(),
        );
    }
    CBoxedSlice::new(usage)
}

#[no_mangle]
pub extern "C" fn luisa_compute_ir_new_node(pools: CArc<ModulePools>, node: Node) -> NodeRef {
    new_node(&pools, node)
}

#[no_mangle]
pub extern "C" fn luisa_compute_ir_node_get(node_ref: NodeRef) -> *const Node {
    node_ref.get()
}

#[no_mangle]
pub extern "C" fn luisa_compute_ir_append_node(builder: &mut IrBuilder, node_ref: NodeRef) {
    builder.append(node_ref)
}

#[no_mangle]
pub extern "C" fn luisa_compute_ir_build_call(
    builder: &mut IrBuilder,
    func: Func,
    args: CSlice<NodeRef>,
    ret_type: CArc<Type>,
) -> NodeRef {
    let args = args.as_ref();
    builder.call(func, args, ret_type)
}

#[no_mangle]
pub extern "C" fn luisa_compute_ir_build_const(builder: &mut IrBuilder, const_: Const) -> NodeRef {
    builder.const_(const_)
}

#[no_mangle]
pub extern "C" fn luisa_compute_ir_build_update(
    builder: &mut IrBuilder,
    var: NodeRef,
    value: NodeRef,
) {
    builder.update(var, value)
}

#[no_mangle]
pub extern "C" fn luisa_compute_ir_build_local(builder: &mut IrBuilder, init: NodeRef) -> NodeRef {
    builder.local(init)
}

#[no_mangle]
pub extern "C" fn luisa_compute_ir_build_local_zero_init(
    builder: &mut IrBuilder,
    ty: CArc<Type>,
) -> NodeRef {
    builder.local_zero_init(ty)
}

#[no_mangle]
pub extern "C" fn luisa_compute_ir_new_module_pools() -> *mut CArcSharedBlock<ModulePools> {
    CArc::into_raw(CArc::new(ModulePools::new()))
}

#[no_mangle]
pub extern "C" fn luisa_compute_ir_new_builder(pools: CArc<ModulePools>) -> IrBuilder {
    unsafe { IrBuilder::new(pools.clone()) }
}

#[no_mangle]
pub extern "C" fn luisa_compute_ir_build_finish(builder: IrBuilder) -> Pooled<BasicBlock> {
    builder.finish()
}

#[no_mangle]
pub extern "C" fn luisa_compute_ir_new_instruction(
    inst: Instruction,
) -> *mut CArcSharedBlock<Instruction> {
    CArc::into_raw(CArc::new(inst))
}

#[no_mangle]
pub extern "C" fn luisa_compute_ir_new_callable_module(m: CallableModule) -> CallableModuleRef {
    CallableModuleRef(CArc::new(m))
}

#[no_mangle]
pub extern "C" fn luisa_compute_ir_new_kernel_module(
    m: KernelModule,
) -> *mut CArcSharedBlock<KernelModule> {
    CArc::into_raw(CArc::new(m))
}

#[no_mangle]
pub extern "C" fn luisa_compute_ir_new_block_module(
    m: BlockModule,
) -> *mut CArcSharedBlock<BlockModule> {
    CArc::into_raw(CArc::new(m))
}

#[no_mangle]
pub extern "C" fn luisa_compute_ir_register_type(ty: &Type) -> *mut CArcSharedBlock<Type> {
    CArc::into_raw(context::register_type(ty.clone()))
}

pub mod debug {
    use crate::display::DisplayIR;
    use std::ffi::CString;

    use super::*;

    pub fn dump_ir_json(module: &Module) -> serde_json::Value {
        serde_json::to_value(&module).unwrap()
    }

    pub fn dump_ir_binary(module: &Module) -> Vec<u8> {
        bincode::serialize(module).unwrap()
    }

    pub fn dump_ir_human_readable(module: &Module) -> String {
        let mut d = DisplayIR::new();
        d.display_ir(module)
    }

    #[no_mangle]
    pub extern "C" fn luisa_compute_ir_dump_json(module: &Module) -> CBoxedSlice<u8> {
        let json = dump_ir_json(module);
        let s = serde_json::to_string(&json).unwrap();
        let cstring = CString::new(s).unwrap();
        CBoxedSlice::new(cstring.as_bytes().to_vec())
    }

    #[no_mangle]
    pub extern "C" fn luisa_compute_ir_dump_binary(module: &Module) -> CBoxedSlice<u8> {
        CBoxedSlice::new(dump_ir_binary(module))
    }

    #[no_mangle]
    pub extern "C" fn luisa_compute_ir_dump_human_readable(module: &Module) -> CBoxedSlice<u8> {
        let mut d = DisplayIR::new();
        let s = d.display_ir(module);
        let cstring = CString::new(s).unwrap();
        CBoxedSlice::new(cstring.as_bytes().to_vec())
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn test_layout() {
        assert_eq!(std::mem::size_of::<super::NodeRef>(), 8);
    }
}
