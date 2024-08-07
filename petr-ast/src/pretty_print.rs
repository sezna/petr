//! Pretty-print the AST for tests and ease of development.

use petr_utils::{PrettyPrint, SymbolInterner};

use crate::*;

impl PrettyPrint for Ast {
    fn pretty_print(
        &self,
        interner: &SymbolInterner,
        _indentation: usize,
    ) -> String {
        let mut buf = String::new();
        for node in &self.modules {
            buf.push_str(&node.pretty_print(interner, 0));
        }
        buf
    }
}

impl PrettyPrint for Module {
    fn pretty_print(
        &self,
        interner: &SymbolInterner,
        indentation: usize,
    ) -> String {
        let mut buf = format!("{}module {} =\n", "  ".repeat(indentation), interner.get_path(&self.name).join("."));
        for node in &self.nodes {
            buf.push_str(&node.pretty_print(interner, 0));
        }
        buf
    }
}

impl PrettyPrint for ImportStatement {
    fn pretty_print(
        &self,
        interner: &SymbolInterner,
        indentation: usize,
    ) -> String {
        let mut buf = format!(
            "{}{} {}",
            "  ".repeat(indentation),
            if self.is_exported() { "export" } else { "import" },
            self.path.iter().map(|id| interner.get(id.id)).collect::<Vec<_>>().join("."),
        );
        if let Some(alias) = self.alias {
            buf.push_str(&format!(" as {}", interner.get(alias.id)));
        }
        buf.push('\n');
        buf
    }
}

impl PrettyPrint for AstNode {
    fn pretty_print(
        &self,
        interner: &SymbolInterner,
        indentation: usize,
    ) -> String {
        let mut string = match self {
            AstNode::FunctionDeclaration(node) => node.pretty_print(interner, indentation),
            AstNode::TypeDeclaration(ty) => ty.pretty_print(interner, indentation),
            AstNode::ImportStatement(stmt) => stmt.pretty_print(interner, indentation),
        };
        let indentation_str = "  ".repeat(indentation);
        string = format!("{indentation_str}{string}");
        let indentation_str = format!("\n{indentation_str}");
        string = string.replace('\n', &indentation_str);
        string
    }
}

impl PrettyPrint for TypeDeclaration {
    fn pretty_print(
        &self,
        interner: &SymbolInterner,
        indentation: usize,
    ) -> String {
        let TypeDeclaration { name, variants, visibility } = self;
        format!(
            "{}{}type {} =\n{}",
            "  ".repeat(indentation),
            if *visibility == Visibility::Exported { "exported " } else { "" },
            name.pretty_print(interner, 0),
            variants
                .iter()
                .map(|field| field.pretty_print(interner, indentation + 1))
                .collect::<Vec<_>>()
                .join(" |\n"),
        )
    }
}

impl PrettyPrint for TypeVariantOrLiteral {
    fn pretty_print(
        &self,
        interner: &SymbolInterner,
        indentation: usize,
    ) -> String {
        match self {
            TypeVariantOrLiteral::Variant(variant) => variant.pretty_print(interner, indentation),
            TypeVariantOrLiteral::Literal(literal) => literal.pretty_print(interner, indentation),
        }
    }
}

impl PrettyPrint for TypeVariant {
    fn pretty_print(
        &self,
        interner: &SymbolInterner,
        indentation: usize,
    ) -> String {
        format!(
            "{}{}({})",
            "  ".repeat(indentation),
            self.name.pretty_print(interner, 0),
            self.fields
                .iter()
                .map(|field| {
                    let name = field.item().name.pretty_print(interner, 0);
                    let ty = field.item().ty.pretty_print(interner, 0);
                    format!("{}: {}", name, ty)
                })
                .collect::<Vec<_>>()
                .join(" ")
        )
    }
}

impl PrettyPrint for Ty {
    fn pretty_print(
        &self,
        interner: &SymbolInterner,
        _: usize,
    ) -> String {
        let name = match self {
            Ty::Bool => "bool".to_string(),
            Ty::Int => "int".to_string(),
            Ty::String => "string".to_string(),
            Ty::Unit => "unit".to_string(),
            Ty::Named(name) => name.pretty_print(interner, 0),
            Ty::Literal(lit) => format!("lit ty {}", lit.pretty_print(interner, 0)),
            Ty::Sum(tys) => tys.iter().map(|ty| ty.pretty_print(interner, 0)).collect::<Vec<_>>().join(" | "),
        };
        format!("'{name}")
    }
}

impl PrettyPrint for FunctionParameter {
    fn pretty_print(
        &self,
        interner: &SymbolInterner,
        indentation: usize,
    ) -> String {
        format!(
            "{}{} ∈ {}",
            "  ".repeat(indentation),
            self.name.pretty_print(interner, 0),
            self.ty.pretty_print(interner, 0)
        )
    }
}

impl PrettyPrint for Expression {
    fn pretty_print(
        &self,
        interner: &SymbolInterner,
        indentation: usize,
    ) -> String {
        match self {
            Expression::Literal(l) => l.pretty_print(interner, indentation),
            Expression::List(list) => list.pretty_print(interner, indentation),
            Expression::Operator(op) => op.pretty_print(interner, indentation),
            Expression::TypeConstructor(..) => "type constructor".to_string(),
            Expression::FunctionCall(call) => call.pretty_print(interner, indentation),
            Expression::Variable(v) => format!("var({})", interner.get(v.id)),
            Expression::IntrinsicCall(call) => call.pretty_print(interner, indentation),
            Expression::Binding(binding) => binding.pretty_print(interner, indentation + 1),
            Expression::If(if_expr) => if_expr.pretty_print(interner, indentation),
        }
    }
}

impl PrettyPrint for Literal {
    fn pretty_print(
        &self,
        _: &SymbolInterner,
        _: usize,
    ) -> String {
        match self {
            Literal::Integer(i) => i.to_string(),
            Literal::Boolean(b) => b.to_string(),
            Literal::String(s) => format!("\"{s}\""),
        }
    }
}

impl PrettyPrint for If {
    fn pretty_print(
        &self,
        interner: &SymbolInterner,
        indentation: usize,
    ) -> String {
        let If {
            condition,
            then_branch,
            else_branch,
        } = self;
        format!(
            "{}if {} then {}{}",
            "  ".repeat(indentation),
            condition.pretty_print(interner, indentation),
            then_branch.pretty_print(interner, indentation),
            if let Some(else_branch) = else_branch {
                format!(" else {}", else_branch.pretty_print(interner, indentation))
            } else {
                Default::default()
            }
        )
    }
}

impl PrettyPrint for ExpressionWithBindings {
    fn pretty_print(
        &self,
        interner: &SymbolInterner,
        indentation: usize,
    ) -> String {
        let mut bindings = self.bindings.iter();
        let Some(first_binding) = bindings.next() else {
            return Default::default();
        };
        let mut buf = format!(
            "\n{}let {} = {},",
            "  ".repeat(indentation),
            first_binding.name.pretty_print(interner, indentation + 1),
            first_binding.val.pretty_print(interner, indentation + 1)
        );
        for (ix, binding) in bindings.enumerate() {
            let is_last = ix == self.bindings.len() - 2;
            buf.push_str(&format!(
                "\n{}    {} = {}{}",
                "  ".repeat(indentation),
                binding.name.pretty_print(interner, indentation + 1),
                binding.val.pretty_print(interner, indentation + 1),
                if !is_last { "," } else { "" }
            ));
        }
        buf.push_str(&format!("\n{}\n\n", self.expression.pretty_print(interner, indentation)));

        buf
    }
}

impl PrettyPrint for IntrinsicCall {
    fn pretty_print(
        &self,
        interner: &SymbolInterner,
        indentation: usize,
    ) -> String {
        format!(
            "{}{}({})",
            "  ".repeat(indentation),
            self.intrinsic,
            self.args
                .iter()
                .map(|arg| arg.pretty_print(interner, indentation))
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}

impl PrettyPrint for FunctionCall {
    fn pretty_print(
        &self,
        interner: &SymbolInterner,
        indentation: usize,
    ) -> String {
        format!(
            "{}call {}({})",
            "  ".repeat(indentation),
            interner.get_path(&self.func_name).join("."),
            self.args
                .iter()
                .map(|arg| arg.pretty_print(interner, indentation))
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}

impl PrettyPrint for List {
    fn pretty_print(
        &self,
        interner: &SymbolInterner,
        indentation: usize,
    ) -> String {
        format!(
            "[{}]",
            self.elements
                .iter()
                .map(|item| item.pretty_print(interner, indentation))
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}

impl PrettyPrint for OperatorExpression {
    fn pretty_print(
        &self,
        interner: &SymbolInterner,
        indentation: usize,
    ) -> String {
        let lhs = self.lhs.pretty_print(interner, indentation);
        let rhs = self.rhs.pretty_print(interner, indentation);
        let op = match self.op.item() {
            Operator::Plus => "+",
            Operator::Minus => "-",
            Operator::Star => "*",
            Operator::Slash => "/",
            Operator::Eq => "=",
        };
        format!("{}({} {})", op, lhs, rhs)
    }
}

impl PrettyPrint for FunctionDeclaration {
    fn pretty_print(
        &self,
        interner: &SymbolInterner,
        indentation: usize,
    ) -> String {
        let FunctionDeclaration {
            name,
            parameters,
            return_type,
            body,
            visibility,
        } = self;
        format!(
            "{}{}Func {}({}{}{}) -> {} {}\n",
            "  ".repeat(indentation),
            if *visibility == Visibility::Exported { "exported " } else { "" },
            name.pretty_print(interner, 0),
            if parameters.is_empty() { "" } else { "\n" },
            parameters
                .iter()
                .map(|param| param.pretty_print(interner, indentation + 1))
                .collect::<Vec<_>>()
                .join(",\n"),
            if parameters.is_empty() { "" } else { "\n" },
            return_type.pretty_print(interner, indentation),
            body.pretty_print(interner, indentation)
        )
    }
}
