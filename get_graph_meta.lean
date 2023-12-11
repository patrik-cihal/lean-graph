import Lean
import Lean.Data.Json.FromToJson
open Lean Elab Term


def getExpr (x : MetaM Syntax) : TermElabM Expr := do
  let synt ← x
  elabTerm synt none

def getConstType (n : Name) : TermElabM String := do
  let constInfo ← getConstInfo n
  return match constInfo with
    | ConstantInfo.defnInfo _ => "Definition"
    | ConstantInfo.thmInfo _  => "Theorem"
    | ConstantInfo.axiomInfo _ => "Axiom"
    | _ => "Other"

def getConstantBody (n : Name) : MetaM (Option Expr) := do
  let constInfo ← getConstInfo n
  let constValue := constInfo.value?
  return constValue


def getAllConstsFromConst (n : Name) := do
  let body ← getConstantBody n
  return (match body with
  | some body => body.getUsedConstants
  | none => {})

structure BFSState :=
  (g : HashMap Name (List Name))
  (outerLayer : List Name)

def getUsedConstantGraph (name : Name) : TermElabM (List (Name × List Name)) := do

  -- make bfs from the specified root node

  -- the goal is to construct a hashmap where the key is the name of the const, and the entry is a list of names of other consts

  -- we keep a list of const names representing the outer layer of the bfs

  -- in each iteration we for each const in the outer layer find its references and that way construct the nodes that will be added to the graph

  -- then we extract the outer layer from the new nodes by looking at the children and checking whether they are already in the graph


  let state ← (List.range 10).foldlM (fun (state : BFSState) (i : Nat) => do
    let g := state.g
    let outerLayer := state.outerLayer

    let newNodes ← outerLayer.mapM fun name => do
      let consts ← getAllConstsFromConst name
      return (name, consts)

    let g := newNodes.foldl (fun m p => m.insert p.fst p.snd.toList) g
    let newOuterLayer := newNodes.foldl (fun (set : HashSet Name) (node : Name × Array Name) =>
      let set := set.insertMany node.snd;
      set) mkHashSet
    let newOuterLayer := newOuterLayer.toList.filter (fun n => !(g.contains n))

    return BFSState.mk g newOuterLayer
  )
    (BFSState.mk mkHashMap [name])




  return state.g.toList


#synth ToJson (List (Name × List Name))

def writeJsonToFile (filePath : String) (json : Json) : IO Unit := do
  let jsonString := toString json
  IO.FS.withFile filePath IO.FS.Mode.write fun handle => do
    handle.putStr jsonString

-- Convert a Name to a String
def nameToString (n : Name) : String :=
  toString n

-- Convert a Name and List Name pair to JSON
def pairToJson (pair : Name × List Name) : TermElabM Json := do
  let nameStr := nameToString pair.fst
  let typeStr ← (getConstType pair.fst)
  let nameListStr := pair.snd.map nameToString
  return Json.mkObj [("name", Json.str nameStr),("constType", Json.str typeStr), ("references", Json.arr (nameListStr.map Json.str).toArray)]

-- Serialize a List (Name, List Name) to JSON
def serializeList (l : List (Name × List Name)) : TermElabM Json := do
  let res ← (l.mapM pairToJson)
  return Json.arr res.toArray

def serializeAndWriteToFile (s : MetaM Syntax) := do
  let expr ← getExpr s
  let name := expr.constName!

  let g ← getUsedConstantGraph name
  let js ←  serializeList g
  let _ ← writeJsonToFile ((toString name).append ".json") js









-- In the line below, specify the constant and uncomment it to get the JSON file

#eval serializeAndWriteToFile `(Nat.zero_add)
