pub trait DartConvertible {
    fn to_dart() -> &'static str;
}

/// Overkilling a simple task, As simple as creating a template file and replacing some placeholders :)
pub struct DartClass {
    pub name: String,
    pub fields: Vec<DartField>,
    pub constructors: Vec<DartConstructor>,
    pub methods: Vec<DartMethod>,
}

impl ToString for DartClass {
    fn to_string(&self) -> String {
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
            "class {} {{\n\t{}\n\n\t{}\n\n\t{}\n}}",
            self.name, fields, constructors, methods
        )
    }
}

pub struct DartField {
    pub keywords: Vec<String>,
    pub name: String,
    pub type_: DartType,
}

impl ToString for DartField {
    fn to_string(&self) -> String {
        let keywords = self.keywords.join(" ");
        format!("{} {} {};", keywords, self.type_.to_string(), self.name)
    }
}

pub enum DartType {
    Primitive(String),
    Class(String),
    List(String),
    Map(String, String),
}

impl ToString for DartType {
    fn to_string(&self) -> String {
        match self {
            DartType::Primitive(name) => name.to_string(),
            DartType::Class(name) => name.to_string(),
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

pub struct DartOnelineConstructor {
    pub name: String,
    pub parameters: DartParameters,
}

impl ToString for DartOnelineConstructor {
    fn to_string(&self) -> String {
        format!("{} ({});", self.name, self.parameters.to_string())
    }
}

pub enum DartParameters {
    Named(Vec<NamedDartParameter>),
    Positional(Vec<DartParameter>),
}

impl ToString for DartParameters {
    fn to_string(&self) -> String {
        match self {
            DartParameters::Named(named) => {
                let params = named
                    .iter()
                    .map(|parameter| parameter.to_string())
                    .collect::<Vec<String>>()
                    .join(", ");
                format!("{{ {} }}", params)
            }
            DartParameters::Positional(positional) => {
                let params = positional
                    .iter()
                    .map(|parameter| parameter.to_string())
                    .collect::<Vec<String>>()
                    .join(", ");
                params
            }
        }
    }
}

pub enum NamedDartParameter {
    Required(DartParameter),
    Optional(DartParameter),
}

impl ToString for NamedDartParameter {
    fn to_string(&self) -> String {
        match self {
            NamedDartParameter::Required(parameter) => {
                format! {"required {}", parameter.to_string()}
            }
            NamedDartParameter::Optional(parameter) => parameter.to_string(),
        }
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

pub struct DartConstructorParameter {
    pub name: String,
}

impl ToString for DartConstructorParameter {
    fn to_string(&self) -> String {
        format!("this.{}", self.name)
    }
}

pub struct DartMethodParameter {
    pub name: String,
    pub type_: DartType,
}

impl ToString for DartMethodParameter {
    fn to_string(&self) -> String {
        format!("{} {}", self.type_.to_string(), self.name)
    }
}
