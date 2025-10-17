module attributes {veridise.lang = "llzk"} {
  struct.def @Signal<[]> {
    struct.field @reg : !felt.type {llzk.pub}
    function.def @compute(%arg0: !felt.type) -> !struct.type<@Signal<[]>> attributes {function.allow_witness} {
      %self = struct.new : <@Signal<[]>>
      struct.writef %self[@reg] = %arg0 : <@Signal<[]>>, !felt.type
      function.return %self : !struct.type<@Signal<[]>>
    }
    function.def @constrain(%arg0: !struct.type<@Signal<[]>>, %arg1: !felt.type) attributes {function.allow_constraint} {
      function.return
    }
  }
  struct.def @Main<[]> {
    struct.field @out_0 : !felt.type {llzk.pub}
    function.def @compute(%arg0: !struct.type<@Signal<[]>> {llzk.pub = #llzk.pub}) -> !struct.type<@Main<[]>> attributes {function.allow_witness} {
      %self = struct.new : <@Main<[]>>
      function.return %self : !struct.type<@Main<[]>>
    }
    function.def @constrain(%arg0: !struct.type<@Main<[]>>, %arg1: !struct.type<@Signal<[]>> {llzk.pub = #llzk.pub}) attributes {function.allow_constraint} {
      %felt_1 = felt.const  1
      %felt_minus_1 = felt.const  21888242871839275222246405745257275088548364400416034343698204186575808495616
      %0 = struct.readf %arg0[@adv_0_0] : <@Main<[]>>, !felt.type
      %1 = felt.mul %felt_minus_1, %0 : !felt.type, !felt.type
      %2 = struct.readf %arg0[@adv_1_0] : <@Main<[]>>, !felt.type 
      %3 = felt.neg %2 : !felt.type 
      %4 = felt.add %1, %3 : !felt.type, !felt.type 
      %5 = felt.mul %felt_1, %4 : !felt.type, !felt.type
      %felt_0 = felt.const  0
      constrain.eq %5, %felt_0 : !felt.type, !felt.type 
      %felt_1_0 = felt.const  1
      %6 = struct.readf %arg0[@adv_0_0] : <@Main<[]>>, !felt.type 
      %7 = struct.readf %arg0[@adv_1_0] : <@Main<[]>>, !felt.type 
      %8 = felt.mul %6, %7 : !felt.type, !felt.type 
      %9 = struct.readf %arg0[@adv_2_0] : <@Main<[]>>, !felt.type 
      %10 = felt.neg %9 : !felt.type 
      %11 = felt.add %8, %10 : !felt.type, !felt.type 
      %12 = felt.mul %felt_1_0, %11 : !felt.type, !felt.type 
      %felt_0_1 = felt.const  0
      constrain.eq %12, %felt_0_1 : !felt.type, !felt.type
      %13 = struct.readf %arg0[@adv_0_0] : <@Main<[]>>, !felt.type 
      %14 = struct.readf %arg1[@reg] : <@Signal<[]>>, !felt.type 
      constrain.eq %13, %14 : !felt.type, !felt.type 
      %15 = struct.readf %arg0[@adv_2_0] : <@Main<[]>>, !felt.type 
      %16 = struct.readf %arg0[@out_0] : <@Main<[]>>, !felt.type 
      constrain.eq %15, %16 : !felt.type, !felt.type 
      function.return
    }
    struct.field @adv_0_0 : !felt.type 
    struct.field @adv_1_0 : !felt.type 
    struct.field @adv_2_0 : !felt.type 
  }
}

