use std::borrow::Cow;
use std::ops::Deref;

use cafebabe::attributes::{AnnotationElementValue, AttributeData};
use cafebabe::descriptors::{FieldDescriptor, FieldType};
use cafebabe::ClassFile;

use super::type_resolver;
use crate::models::MethodInfo;

/// Spring 注解的类名 (JVM 内部格式)
const CONTROLLER_CLASS: &str = "org/springframework/stereotype/Controller";
const REST_CONTROLLER_CLASS: &str = "org/springframework/web/bind/annotation/RestController";
const REQUEST_MAPPING_CLASS: &str = "org/springframework/web/bind/annotation/RequestMapping";
const GET_MAPPING_CLASS: &str = "org/springframework/web/bind/annotation/GetMapping";
const POST_MAPPING_CLASS: &str = "org/springframework/web/bind/annotation/PostMapping";
const PUT_MAPPING_CLASS: &str = "org/springframework/web/bind/annotation/PutMapping";
const DELETE_MAPPING_CLASS: &str = "org/springframework/web/bind/annotation/DeleteMapping";
const PATCH_MAPPING_CLASS: &str = "org/springframework/web/bind/annotation/PatchMapping";
const API_OPERATION_CLASS: &str = "io/swagger/annotations/ApiOperation";
const API_MODEL_PROPERTY_CLASS: &str = "io/swagger/annotations/ApiModelProperty";

/// 检查类是否有 @Controller 或 @RestController
pub fn is_controller(class: &ClassFile) -> bool {
    has_class_annotation(class, CONTROLLER_CLASS)
        || has_class_annotation(class, REST_CONTROLLER_CLASS)
}

fn has_class_annotation(class: &ClassFile, target_class: &str) -> bool {
    class
        .attributes
        .iter()
        .filter_map(|attr| match &attr.data {
            AttributeData::RuntimeVisibleAnnotations(annotations) => Some(annotations),
            _ => None,
        })
        .flatten()
        .any(|ann| is_annotation_type(&ann.type_descriptor, target_class))
}

/// 判断 FieldDescriptor 是否匹配指定注解类
fn is_annotation_type(fd: &FieldDescriptor, target_class: &str) -> bool {
    match &fd.field_type {
        FieldType::Object(cn) => cn.deref() == target_class,
        _ => false,
    }
}

/// 获取类级别 @RequestMapping 路径
pub fn get_class_request_mapping(class: &ClassFile) -> String {
    class
        .attributes
        .iter()
        .filter_map(|attr| match &attr.data {
            AttributeData::RuntimeVisibleAnnotations(annotations) => Some(annotations),
            _ => None,
        })
        .flatten()
        .find(|ann| is_annotation_type(&ann.type_descriptor, REQUEST_MAPPING_CLASS))
        .and_then(|ann| {
            get_annotation_string_value(ann, "value")
                .or_else(|| get_annotation_string_value(ann, "path"))
        })
        .unwrap_or_default()
}

/// 获取方法上的 HTTP 注解信息
pub fn get_http_methods(class: &ClassFile) -> Vec<MethodInfo> {
    let mut methods = Vec::new();

    for method in &class.methods {
        for attr in &method.attributes {
            if let AttributeData::RuntimeVisibleAnnotations(annotations) = &attr.data {
                for ann in annotations {
                    if let Some(http_method) = http_method_from_descriptor(&ann.type_descriptor) {
                        let path = get_annotation_string_value(ann, "value")
                            .or_else(|| get_annotation_string_value(ann, "path"))
                            .unwrap_or_default();
                        let api_name = get_api_operation_name(annotations);
                        methods.push(MethodInfo {
                            method_name: method.name.to_string(),
                            http_method,
                            path,
                            api_name,
                        });
                    }
                }
            }
        }
    }

    methods
}

fn http_method_from_descriptor(fd: &FieldDescriptor) -> Option<String> {
    let class_name = match &fd.field_type {
        FieldType::Object(cn) => cn.deref(),
        _ => return None,
    };

    match class_name {
        GET_MAPPING_CLASS => Some("GET".to_string()),
        POST_MAPPING_CLASS => Some("POST".to_string()),
        PUT_MAPPING_CLASS => Some("PUT".to_string()),
        DELETE_MAPPING_CLASS => Some("DELETE".to_string()),
        PATCH_MAPPING_CLASS => Some("PATCH".to_string()),
        REQUEST_MAPPING_CLASS => Some("GET".to_string()),
        _ => None,
    }
}

/// 从注解元素中获取字符串值
pub fn get_annotation_string_value(
    ann: &cafebabe::attributes::Annotation,
    key: &str,
) -> Option<String> {
    for element in &ann.elements {
        if element.name == key {
            return extract_string_from_element(&element.value);
        }
    }
    None
}

/// 从注解元素值中提取字符串
fn extract_string_from_element(value: &AnnotationElementValue) -> Option<String> {
    match value {
        AnnotationElementValue::StringConstant(s) => Some(s.to_string()),
        AnnotationElementValue::ArrayValue(values) => {
            values.first().and_then(|v| extract_string_from_element(v))
        }
        AnnotationElementValue::EnumConstant { const_name, .. } => Some(const_name.to_string()),
        _ => None,
    }
}

/// 从方法注解中获取 @ApiOperation 的 value
fn get_api_operation_name(annotations: &[cafebabe::attributes::Annotation]) -> String {
    annotations
        .iter()
        .find(|ann| is_annotation_type(&ann.type_descriptor, API_OPERATION_CLASS))
        .and_then(|ann| get_annotation_string_value(ann, "value"))
        .unwrap_or_default()
}

/// 获取字段上的 @ApiModelProperty 信息
/// Returns (comment, required, example_value)
pub fn get_api_model_property(class: &ClassFile, field_name: &str) -> (String, String, String) {
    for field in &class.fields {
        if field.name == field_name {
            for attr in &field.attributes {
                if let AttributeData::RuntimeVisibleAnnotations(annotations) = &attr.data {
                    for ann in annotations {
                        if is_annotation_type(&ann.type_descriptor, API_MODEL_PROPERTY_CLASS) {
                            let comment =
                                get_annotation_string_value(ann, "value").unwrap_or_default();
                            let required = get_annotation_bool_value(ann, "required")
                                .map(|b| if b { "是" } else { "否" })
                                .unwrap_or("否")
                                .to_string();
                            let example =
                                get_annotation_string_value(ann, "example").unwrap_or_default();
                            return (comment, required, example);
                        }
                    }
                }
            }
        }
    }
    (String::new(), "否".to_string(), String::new())
}

fn get_annotation_bool_value(ann: &cafebabe::attributes::Annotation, key: &str) -> Option<bool> {
    for element in &ann.elements {
        if element.name == key {
            return match &element.value {
                AnnotationElementValue::BooleanConstant(b) => Some(*b != 0),
                AnnotationElementValue::IntConstant(i) => Some(*i != 0),
                _ => None,
            };
        }
    }
    None
}

pub fn get_field_type_name(fd: &FieldDescriptor) -> String {
    let base = match &fd.field_type {
        FieldType::Byte => "byte".to_string(),
        FieldType::Char => "char".to_string(),
        FieldType::Double => "Double".to_string(),
        FieldType::Float => "Float".to_string(),
        FieldType::Integer => "Integer".to_string(),
        FieldType::Long => "Long".to_string(),
        FieldType::Short => "Short".to_string(),
        FieldType::Boolean => "Boolean".to_string(),
        FieldType::Object(cn) => type_resolver::simplify_class_name(cn.deref()),
    };

    let brackets = "[]".repeat(fd.dimensions as usize);
    format!("{}{}", base, brackets)
}

/// 获取方法签名中的泛型签名 (从 Signature 属性)
pub fn get_method_generic_signature(class: &ClassFile, method_name: &str) -> Option<String> {
    for method in &class.methods {
        if method.name == method_name {
            for attr in &method.attributes {
                if let AttributeData::Signature(sig) = &attr.data {
                    return Some(sig.to_string());
                }
            }
        }
    }
    None
}

/// 获取类的 Signature 属性
pub fn get_class_signature(class: &ClassFile) -> Option<String> {
    for attr in &class.attributes {
        if let AttributeData::Signature(sig) = &attr.data {
            return Some(sig.to_string());
        }
    }
    None
}

/// 获取字段的 Signature 属性
pub fn get_field_signature<'a>(class: &'a ClassFile, field_name: &str) -> Option<Cow<'a, str>> {
    for field in &class.fields {
        if field.name == field_name {
            for attr in &field.attributes {
                if let AttributeData::Signature(sig) = &attr.data {
                    return Some(sig.clone());
                }
            }
        }
    }
    None
}
