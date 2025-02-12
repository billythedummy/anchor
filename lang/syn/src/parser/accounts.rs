use crate::{
    AccountField, AccountsStruct, CompositeField, Constraint, ConstraintAssociated,
    ConstraintBelongsTo, ConstraintExecutable, ConstraintLiteral, ConstraintOwner,
    ConstraintRentExempt, ConstraintSeeds, ConstraintSigner, ConstraintState, CpiAccountTy,
    CpiStateTy, Field, LoaderTy, ProgramAccountTy, ProgramStateTy, SysvarTy, Ty,
};

pub fn parse(strct: &syn::ItemStruct) -> AccountsStruct {
    let fields = match &strct.fields {
        syn::Fields::Named(fields) => fields.named.iter().map(parse_account_field).collect(),
        _ => panic!("invalid input"),
    };
    AccountsStruct::new(strct.clone(), fields)
}

fn parse_account_field(f: &syn::Field) -> AccountField {
    let anchor_attr = parse_account_attr(f);
    parse_field(f, anchor_attr)
}

fn parse_account_attr(f: &syn::Field) -> Option<&syn::Attribute> {
    let anchor_attrs: Vec<&syn::Attribute> = f
        .attrs
        .iter()
        .filter(|attr| {
            if attr.path.segments.len() != 1 {
                return false;
            }
            if attr.path.segments[0].ident != "account" {
                return false;
            }
            true
        })
        .collect();
    match anchor_attrs.len() {
        0 => None,
        1 => Some(anchor_attrs[0]),
        _ => panic!("Invalid syntax: please specify one account attribute."),
    }
}

fn parse_field(f: &syn::Field, anchor: Option<&syn::Attribute>) -> AccountField {
    let ident = f.ident.clone().unwrap();
    let (constraints, is_mut, is_signer, is_init, payer, space, associated_seeds) = match anchor {
        None => (vec![], false, false, false, None, None, Vec::new()),
        Some(anchor) => parse_constraints(anchor),
    };
    match is_field_primitive(f) {
        true => {
            let ty = parse_ty(f);
            AccountField::Field(Field {
                ident,
                ty,
                constraints,
                is_mut,
                is_signer,
                is_init,
                payer,
                space,
                associated_seeds,
            })
        }
        false => AccountField::AccountsStruct(CompositeField {
            ident,
            symbol: ident_string(f),
            constraints,
            raw_field: f.clone(),
        }),
    }
}

fn is_field_primitive(f: &syn::Field) -> bool {
    match ident_string(f).as_str() {
        "ProgramState" | "ProgramAccount" | "CpiAccount" | "Sysvar" | "AccountInfo"
        | "CpiState" | "Loader" => true,
        _ => false,
    }
}

fn parse_ty(f: &syn::Field) -> Ty {
    let path = match &f.ty {
        syn::Type::Path(ty_path) => ty_path.path.clone(),
        _ => panic!("invalid account syntax"),
    };
    match ident_string(f).as_str() {
        "ProgramState" => Ty::ProgramState(parse_program_state(&path)),
        "CpiState" => Ty::CpiState(parse_cpi_state(&path)),
        "ProgramAccount" => Ty::ProgramAccount(parse_program_account(&path)),
        "CpiAccount" => Ty::CpiAccount(parse_cpi_account(&path)),
        "Sysvar" => Ty::Sysvar(parse_sysvar(&path)),
        "AccountInfo" => Ty::AccountInfo,
        "Loader" => Ty::Loader(parse_program_account_zero_copy(&path)),
        _ => panic!("invalid account type"),
    }
}

fn ident_string(f: &syn::Field) -> String {
    let path = match &f.ty {
        syn::Type::Path(ty_path) => ty_path.path.clone(),
        _ => panic!("invalid account syntax"),
    };
    // TODO: allow segmented paths.
    assert!(path.segments.len() == 1);
    let segments = &path.segments[0];
    segments.ident.to_string()
}

fn parse_program_state(path: &syn::Path) -> ProgramStateTy {
    let account_ident = parse_account(&path);
    ProgramStateTy { account_ident }
}

fn parse_cpi_state(path: &syn::Path) -> CpiStateTy {
    let account_ident = parse_account(&path);
    CpiStateTy { account_ident }
}

fn parse_cpi_account(path: &syn::Path) -> CpiAccountTy {
    let account_ident = parse_account(path);
    CpiAccountTy { account_ident }
}

fn parse_program_account(path: &syn::Path) -> ProgramAccountTy {
    let account_ident = parse_account(path);
    ProgramAccountTy { account_ident }
}

fn parse_program_account_zero_copy(path: &syn::Path) -> LoaderTy {
    let account_ident = parse_account(path);
    LoaderTy { account_ident }
}

fn parse_account(path: &syn::Path) -> syn::Ident {
    let segments = &path.segments[0];
    match &segments.arguments {
        syn::PathArguments::AngleBracketed(args) => {
            // Expected: <'info, MyType>.
            assert!(args.args.len() == 2);
            match &args.args[1] {
                syn::GenericArgument::Type(syn::Type::Path(ty_path)) => {
                    // TODO: allow segmented paths.
                    assert!(ty_path.path.segments.len() == 1);
                    let path_segment = &ty_path.path.segments[0];
                    path_segment.ident.clone()
                }
                _ => panic!("Invalid ProgramAccount"),
            }
        }
        _ => panic!("Invalid ProgramAccount"),
    }
}

fn parse_sysvar(path: &syn::Path) -> SysvarTy {
    let segments = &path.segments[0];
    let account_ident = match &segments.arguments {
        syn::PathArguments::AngleBracketed(args) => {
            // Expected: <'info, MyType>.
            assert!(args.args.len() == 2);
            match &args.args[1] {
                syn::GenericArgument::Type(syn::Type::Path(ty_path)) => {
                    // TODO: allow segmented paths.
                    assert!(ty_path.path.segments.len() == 1);
                    let path_segment = &ty_path.path.segments[0];
                    path_segment.ident.clone()
                }
                _ => panic!("Invalid Sysvar"),
            }
        }
        _ => panic!("Invalid Sysvar"),
    };
    match account_ident.to_string().as_str() {
        "Clock" => SysvarTy::Clock,
        "Rent" => SysvarTy::Rent,
        "EpochSchedule" => SysvarTy::EpochSchedule,
        "Fees" => SysvarTy::Fees,
        "RecentBlockhashes" => SysvarTy::RecentBlockhashes,
        "SlotHashes" => SysvarTy::SlotHashes,
        "SlotHistory" => SysvarTy::SlotHistory,
        "StakeHistory" => SysvarTy::StakeHistory,
        "Instructions" => SysvarTy::Instructions,
        "Rewards" => SysvarTy::Rewards,
        _ => panic!("Invalid Sysvar"),
    }
}

fn parse_constraints(
    anchor: &syn::Attribute,
) -> (
    Vec<Constraint>,
    bool,
    bool,
    bool,
    Option<syn::Ident>,
    Option<proc_macro2::TokenStream>,
    Vec<syn::Ident>,
) {
    let mut tts = anchor.tokens.clone().into_iter();
    let g_stream = match tts.next().expect("Must have a token group") {
        proc_macro2::TokenTree::Group(g) => g.stream(),
        _ => panic!("Invalid syntax"),
    };

    let mut is_init = false;
    let mut is_mut = false;
    let mut is_signer = false;
    let mut constraints = vec![];
    let mut is_rent_exempt = None;
    let mut payer = None;
    let mut space = None;
    let mut is_associated = false;
    let mut associated_seeds = Vec::new();

    let mut inner_tts = g_stream.into_iter();
    while let Some(token) = inner_tts.next() {
        match token {
            proc_macro2::TokenTree::Ident(ident) => match ident.to_string().as_str() {
                "init" => {
                    is_init = true;
                    is_mut = true;
                    // If it's not specified, all program owned accounts default
                    // to being rent exempt.
                    if is_rent_exempt.is_none() {
                        is_rent_exempt = Some(true);
                    }
                }
                "mut" => {
                    is_mut = true;
                }
                "signer" => {
                    is_signer = true;
                    constraints.push(Constraint::Signer(ConstraintSigner {}));
                }
                "seeds" => {
                    match inner_tts.next().unwrap() {
                        proc_macro2::TokenTree::Punct(punct) => {
                            assert!(punct.as_char() == '=');
                            punct
                        }
                        _ => panic!("invalid syntax"),
                    };
                    let seeds = match inner_tts.next().unwrap() {
                        proc_macro2::TokenTree::Group(g) => g,
                        _ => panic!("invalid syntax"),
                    };
                    constraints.push(Constraint::Seeds(ConstraintSeeds { seeds }))
                }
                "belongs_to" | "has_one" => {
                    match inner_tts.next().unwrap() {
                        proc_macro2::TokenTree::Punct(punct) => {
                            assert!(punct.as_char() == '=');
                            punct
                        }
                        _ => panic!("invalid syntax"),
                    };
                    let join_target = match inner_tts.next().unwrap() {
                        proc_macro2::TokenTree::Ident(ident) => ident,
                        _ => panic!("invalid syntax"),
                    };
                    constraints.push(Constraint::BelongsTo(ConstraintBelongsTo { join_target }))
                }
                "owner" => {
                    match inner_tts.next().unwrap() {
                        proc_macro2::TokenTree::Punct(punct) => {
                            assert!(punct.as_char() == '=');
                            punct
                        }
                        _ => panic!("invalid syntax"),
                    };
                    let owner_target = match inner_tts.next().unwrap() {
                        proc_macro2::TokenTree::Ident(ident) => ident,
                        _ => panic!("invalid syntax"),
                    };
                    constraints.push(Constraint::Owner(ConstraintOwner { owner_target }));
                }
                "rent_exempt" => {
                    match inner_tts.next() {
                        None => is_rent_exempt = Some(true),
                        Some(tkn) => {
                            match tkn {
                                proc_macro2::TokenTree::Punct(punct) => {
                                    assert!(punct.as_char() == '=');
                                    punct
                                }
                                _ => panic!("invalid syntax"),
                            };
                            let should_skip = match inner_tts.next().unwrap() {
                                proc_macro2::TokenTree::Ident(ident) => ident,
                                _ => panic!("invalid syntax"),
                            };
                            match should_skip.to_string().as_str() {
                                "skip" => {
                                    is_rent_exempt = Some(false);
                                },
                                _ => panic!("invalid syntax: omit the rent_exempt attribute to enforce rent exemption"),
                            };
                        }
                    };
                }
                "executable" => {
                    constraints.push(Constraint::Executable(ConstraintExecutable {}));
                }
                "state" => {
                    match inner_tts.next().unwrap() {
                        proc_macro2::TokenTree::Punct(punct) => {
                            assert!(punct.as_char() == '=');
                            punct
                        }
                        _ => panic!("invalid syntax"),
                    };
                    let program_target = match inner_tts.next().unwrap() {
                        proc_macro2::TokenTree::Ident(ident) => ident,
                        _ => panic!("invalid syntax"),
                    };
                    constraints.push(Constraint::State(ConstraintState { program_target }));
                }
                "associated" => {
                    is_associated = true;
                    is_mut = true;
                    match inner_tts.next().unwrap() {
                        proc_macro2::TokenTree::Punct(punct) => {
                            assert!(punct.as_char() == '=');
                            punct
                        }
                        _ => panic!("invalid syntax"),
                    };
                    let associated_target = match inner_tts.next().unwrap() {
                        proc_macro2::TokenTree::Ident(ident) => ident,
                        _ => panic!("invalid syntax"),
                    };
                    constraints.push(Constraint::Associated(ConstraintAssociated {
                        associated_target,
                    }));
                }
                "with" => {
                    match inner_tts.next().unwrap() {
                        proc_macro2::TokenTree::Punct(punct) => {
                            assert!(punct.as_char() == '=');
                            punct
                        }
                        _ => panic!("invalid syntax"),
                    };
                    associated_seeds.push(match inner_tts.next().unwrap() {
                        proc_macro2::TokenTree::Ident(ident) => ident,
                        _ => panic!("invalid syntax"),
                    });
                }
                "payer" => {
                    match inner_tts.next().unwrap() {
                        proc_macro2::TokenTree::Punct(punct) => {
                            assert!(punct.as_char() == '=');
                            punct
                        }
                        _ => panic!("invalid syntax"),
                    };
                    let _payer = match inner_tts.next().unwrap() {
                        proc_macro2::TokenTree::Ident(ident) => ident,
                        _ => panic!("invalid syntax"),
                    };
                    payer = Some(_payer);
                }
                "space" => {
                    match inner_tts.next().unwrap() {
                        proc_macro2::TokenTree::Punct(punct) => {
                            assert!(punct.as_char() == '=');
                            punct
                        }
                        _ => panic!("invalid syntax"),
                    };
                    match inner_tts.next().unwrap() {
                        proc_macro2::TokenTree::Literal(literal) => {
                            let tokens: proc_macro2::TokenStream =
                                literal.to_string().replace("\"", "").parse().unwrap();
                            space = Some(tokens);
                        }
                        _ => panic!("invalid space"),
                    }
                }
                _ => {
                    panic!("invalid syntax");
                }
            },
            proc_macro2::TokenTree::Punct(punct) => {
                if punct.as_char() != ',' {
                    panic!("invalid syntax");
                }
            }
            proc_macro2::TokenTree::Literal(literal) => {
                let tokens: proc_macro2::TokenStream =
                    literal.to_string().replace("\"", "").parse().unwrap();
                constraints.push(Constraint::Literal(ConstraintLiteral { tokens }));
            }
            _ => {
                panic!("invalid syntax");
            }
        }
    }

    // If `associated` is given, remove `init` since it's redundant.
    if is_associated {
        is_init = false;
    }

    if let Some(is_re) = is_rent_exempt {
        match is_re {
            false => constraints.push(Constraint::RentExempt(ConstraintRentExempt::Skip)),
            true => constraints.push(Constraint::RentExempt(ConstraintRentExempt::Enforce)),
        }
    }

    (
        constraints,
        is_mut,
        is_signer,
        is_init,
        payer,
        space,
        associated_seeds,
    )
}
