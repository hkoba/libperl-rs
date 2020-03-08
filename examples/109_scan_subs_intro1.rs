#[cfg(perlapi_ver26)]
use std::env;

#[cfg(perlapi_ver26)]
use libperl_rs::*;

#[cfg(perlapi_ver26)]
use libperl_sys::opcode;

#[cfg(perlapi_ver26)]
mod eg;
#[cfg(perlapi_ver26)]
use eg::{op1::*,sv0::*,cv0::*,stash_walker0::*};

#[cfg(perlapi_ver26)]
fn match_param_list(op: &Op) -> Vec<PadNameType> {
    let mut res: Vec<PadNameType> = Vec::new();
    if let Op::UNOP(opcode::OP_NULL, _,
                    Op::OP(opcode::OP_PUSHMARK, _, _, ref args_op), _) = op {
        let mut args_op = args_op;
        while let Op::OP(_, _, Some(arg), rest) = args_op {
            res.push(arg.clone());
            if let Op::NULL = rest {
                break
            }
            args_op = rest;
        }
    }
    res
}

#[cfg(perlapi_ver26)]
fn my_test() {
    let mut perl = Perl::new();
    perl.parse_env_args(env::args(), env::vars());
    
    let op_extractor = OpExtractor::new(&perl);

    let main_file = sv_extract_pv(perl.get_sv("0", 0)).unwrap();
    println!("$0 = {:?}", main_file);
    
    let filter = |cv| CvFILE(cv).map_or(false, |s| s == main_file);

    let mut emitter = |name: &String, cv: *const libperl_sys::cv| {
        println!("sub {:?}", name);
        let ast = op_extractor.extract(cv, CvROOT(cv));
        
        match ast {
            Op::UNOP(opcode::OP_LEAVESUB, _
                     , Op::LISTOP(opcode::OP_LINESEQ, _
                                  , Op::COP(opcode::OP_NEXTSTATE, body), _), _) => {
                println!("preamble!");
                match body {
                    Op::BINOP(opcode::OP_AASSIGN, _
                              , Op::UNOP(opcode::OP_NULL, _
                                         , Op::OP(opcode::OP_PADRANGE, _, _
                                                  , Op::UNOP(opcode::OP_RV2AV, _
                                                             , Op::PADOP(opcode::OP_GV, _
                                                                         , Sv::GLOB { name: ref nm, .. })
                                                             , _))
                                         , lvalue)
                              , _) if nm == "_" => {
                        println!("first array assignment from @_, lvalue = {:?}"
                                 , match_param_list(lvalue));
                        
                    }
                    _ => {
                        println!("first statement is not an array assignment");
                    }
                }
            }
            _ => {
                println!("doesn't match")
            }
        }

        println!("");
    };

    let mut nswalker = StashWalker::new(&perl, Some(&filter), &mut emitter);

    nswalker.walk("");
}

#[cfg(not(perlapi_ver26))]
fn my_test() {
    println!("Requires perl >= 5.26");
}

fn main() {
    my_test();
}
