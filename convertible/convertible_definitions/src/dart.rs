pub trait DartConvertible {
    fn to_dart() -> &'static str;
}

/// Overkilling a simple task, As simple as creating a template file and replacing some placeholders :)
pub struct DartClass {
    /// @JsonSerializable() a list of Strings for now
    pub decorators: Vec<String>,
    pub name: String,
    pub fields: Vec<DartField>,
    pub constructors: Vec<DartConstructor>,
    pub methods: Vec<DartMethod>,
}

impl ToString for DartClass {
    fn to_string(&self) -> String {
        let decorators = self.decorators.join("\n");

        let fields = self
            .fields
            .iter()
            .map(|field| field.to_string())
            .collect::<Vec<String>>()
            .join("\n\t");

        let constructors = self
            .constructors
            .iter()
            .map(|constructor| constructor.to_string())
            .collect::<Vec<String>>()
            .join("\n\n\t");

        let methods = self
            .methods
            .iter()
            .map(|method| method.to_string())
            .collect::<Vec<String>>()
            .join("\n\n\t");

        format!(
            "{}\nclass {} {{\n\t{}\n\n\t{}\n\n\t{}\n}}",
            decorators, self.name, fields, constructors, methods
        )
    }
}

/// A dart field:
/// final String? id;
pub struct DartField {
    /// Final or const
    pub keywords: Vec<String>,
    pub name: String,
    pub type_: DartType,
    /// Add `?`to the type
    pub optional: bool,
}

impl ToString for DartField {
    fn to_string(&self) -> String {
        let keywords = self.keywords.join(" ");
        let optional_mark = if self.optional { "?" } else { "" };
        format!(
            "{} {}{} {};",
            keywords,
            self.type_.to_string(),
            optional_mark,
            self.name
        )
    }
}

pub enum DartType {
    /// Every type as a string
    Primitive(String),
    List(String),
    Map(String, String),
}

impl ToString for DartType {
    fn to_string(&self) -> String {
        match self {
            DartType::Primitive(name) => name.to_string(),
            DartType::List(name) => format!("List<{}>", name),
            DartType::Map(key, value) => format!("Map<{}, {}>", key, value),
        }
    }
}

pub enum DartConstructor {
    OneLiner(DartOnelineConstructor),
    Factory(DartFactoryConstructor),
}

impl ToString for DartConstructor {
    fn to_string(&self) -> String {
        match self {
            DartConstructor::OneLiner(one_liner) => one_liner.to_string(),
            DartConstructor::Factory(factory) => factory.to_string(),
        }
    }
}

/// A one line constructor:
/// Project ({ required this.id, required this.installed, required this.scripts });
pub struct DartOnelineConstructor {
    pub name: String,
    pub parameters: DartParameters,
}

impl ToString for DartOnelineConstructor {
    fn to_string(&self) -> String {
        format!("{} ({});", self.name, self.parameters.to_string())
    }
}

/// A factory constructor:
/// factory Project.fromJson(Map<String, dynamic> json) => _$ProjectFromJson(json);
pub enum DartFactoryConstructor {
    OneLiner(DartOnelineFactoryConstructor),
}

impl ToString for DartFactoryConstructor {
    fn to_string(&self) -> String {
        match self {
            DartFactoryConstructor::OneLiner(one_liner) => one_liner.to_string(),
        }
    }
}

pub struct DartOnelineFactoryConstructor {
    pub class_name: String,
    pub name: String,
    pub parameters: DartParameters,
    pub body: MethodBody,
}

impl ToString for DartOnelineFactoryConstructor {
    fn to_string(&self) -> String {
        let parameters = self.parameters.to_string();
        let body = match &self.body {
            MethodBody::OneLiner(online) => online.to_string(),
        };
        format!(
            "factory {}.{}({}) => {};",
            self.class_name, self.name, parameters, body
        )
    }
}

pub enum DartMethod {
    OneLiner(DartOnelineMethod),
}

impl ToString for DartMethod {
    fn to_string(&self) -> String {
        match self {
            DartMethod::OneLiner(one_liner) => one_liner.to_string(),
        }
    }
}

/// A one line method:
/// Map<String, dynamic> toJson() => _$ProjectToJson(this);
pub struct DartOnelineMethod {
    pub name: String,
    pub type_: DartType,
    pub parameters: DartParameters,
    pub body: MethodBody,
}

impl ToString for DartOnelineMethod {
    fn to_string(&self) -> String {
        format!(
            "{} {}({}) => {};",
            self.type_.to_string(),
            self.name,
            self.parameters.to_string(),
            self.body.to_string()
        )
    }
}

pub enum MethodBody {
    OneLiner(OnelineMethodBody),
}

impl ToString for MethodBody {
    fn to_string(&self) -> String {
        match self {
            MethodBody::OneLiner(online) => online.to_string(),
        }
    }
}

/// A one line method body with no brackets:
/// _$ProjectToJson(this)
pub struct OnelineMethodBody {
    pub name: String,
    pub parameters: Vec<String>,
}

impl ToString for OnelineMethodBody {
    fn to_string(&self) -> String {
        let parameters = self.parameters.join(", ");
        format!("{}({})", self.name, parameters)
    }
}

pub enum DartParameters {
    Named(Vec<NamedDartParameter>),
    Positional(Vec<DartParameter>),
}

impl ToString for DartParameters {
    fn to_string(&self) -> String {
        fn collect_params<T: ToString>(params: &[T]) -> String {
            params
                .iter()
                .map(|parameter| parameter.to_string())
                .collect::<Vec<String>>()
                .join(", ")
        }

        match self {
            DartParameters::Named(named) => {
                let params = collect_params(named);
                format!("{{ {} }}", params)
            }
            DartParameters::Positional(positional) => collect_params(positional),
        }
    }
}

/// A named parameter:
/// { id, required installed, required scripts }
pub struct NamedDartParameter {
    /// Sets required keyword
    pub required: bool,
    pub parameter: DartParameter,
}

impl ToString for NamedDartParameter {
    fn to_string(&self) -> String {
        let required = if self.required { "required " } else { "" };
        format!("{}{}", required, self.parameter.to_string())
    }
}

pub enum DartParameter {
    ConstructorParameter(DartConstructorParameter),
    MethodParameter(DartMethodParameter),
}

impl ToString for DartParameter {
    fn to_string(&self) -> String {
        match self {
            DartParameter::ConstructorParameter(parameter) => parameter.to_string(),
            DartParameter::MethodParameter(parameter) => parameter.to_string(),
        }
    }
}

/// A constructor parameter:
/// this.id
pub struct DartConstructorParameter {
    pub name: String,
}

impl ToString for DartConstructorParameter {
    fn to_string(&self) -> String {
        format!("this.{}", self.name)
    }
}

/// A method parameter:
/// String id
pub struct DartMethodParameter {
    pub name: String,
    pub type_: DartType,
}

impl ToString for DartMethodParameter {
    fn to_string(&self) -> String {
        format!("{} {}", self.type_.to_string(), self.name)
    }
}

pub fn dev() {
    let fields = vec![
        DartField {
            keywords: vec!["final".into()],
            name: "id".into(),
            type_: DartType::Primitive("String".into()),
            optional: false,
        },
        DartField {
            keywords: vec!["final".into()],
            name: "installed".into(),
            type_: DartType::Primitive("bool".into()),
            optional: false,
        },
        DartField {
            keywords: vec!["final".into()],
            name: "scripts".into(),
            type_: DartType::List("Script".into()),
            optional: false,
        },
    ];

    let cons_parameters = DartParameters::Named(vec![
        NamedDartParameter {
            required: true,
            parameter: DartParameter::ConstructorParameter(DartConstructorParameter {
                name: "id".into(),
            }),
        },
        NamedDartParameter {
            required: true,
            parameter: DartParameter::ConstructorParameter(DartConstructorParameter {
                name: "installed".into(),
            }),
        },
        NamedDartParameter {
            required: true,
            parameter: DartParameter::ConstructorParameter(DartConstructorParameter {
                name: "scripts".into(),
            }),
        },
    ]);

    let constructor = DartConstructor::OneLiner(DartOnelineConstructor {
        name: "Project".into(),
        parameters: cons_parameters,
    });

    let factory_body = MethodBody::OneLiner(OnelineMethodBody {
        name: "_$ProjectFromJson".into(),
        parameters: vec!["json".into()],
    });

    let factory_params =
        DartParameters::Positional(vec![DartParameter::MethodParameter(DartMethodParameter {
            name: "json".into(),
            type_: DartType::Map("String".into(), "dynamic".into()),
        })]);

    let factory = DartConstructor::Factory(DartFactoryConstructor::OneLiner(
        DartOnelineFactoryConstructor {
            class_name: "Project".into(),
            name: "fromJson".into(),
            parameters: factory_params,
            body: factory_body,
        },
    ));

    let to_json_method_params = DartParameters::Positional(vec![]);

    let to_json_method_body = MethodBody::OneLiner(OnelineMethodBody {
        name: "_$ProjectToJson".into(),
        parameters: vec!["this".into()],
    });

    let to_json_method = DartMethod::OneLiner(DartOnelineMethod {
        name: "toJson".into(),
        type_: DartType::Map("String".into(), "dynamic".into()),
        parameters: to_json_method_params,
        body: to_json_method_body,
    });

    let dart_class = DartClass {
        decorators: vec!["@JsonSerializable()".into()],
        name: "Project".into(),
        fields,
        constructors: vec![constructor, factory],
        methods: vec![to_json_method],
    };

    println!("{}", dart_class.to_string());
}
