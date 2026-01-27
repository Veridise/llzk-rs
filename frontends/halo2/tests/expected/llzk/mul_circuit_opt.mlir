module attributes {veridise.lang = "llzk"} {
  struct.def @Signal<[]> {
    struct.field @reg : !felt.type {llzk.pub}
    function.def @compute(%arg0: !felt.type) -> !struct.type<@Signal<[]>> attributes {function.allow_non_native_field_ops, function.allow_witness} {
      %self = struct.new : <@Signal<[]>>
      struct.writef %self[@reg] = %arg0 : <@Signal<[]>>, !felt.type
      function.return %self : !struct.type<@Signal<[]>>
    }
    function.def @constrain(%arg0: !struct.type<@Signal<[]>>, %arg1: !felt.type) attributes {function.allow_constraint, function.allow_non_native_field_ops} {
      %0 = struct.readf %arg0[@reg] : <@Signal<[]>>, !felt.type
      constrain.eq %0, %arg1 : !felt.type, !felt.type
      function.return
    }
  }
  struct.def @Main<[]> {
    struct.field @out_0 : !felt.type {llzk.pub}
    function.def @compute(%arg0: !struct.type<@Signal<[]>> {llzk.pub = #llzk.pub}) -> !struct.type<@Main<[]>> attributes {function.allow_non_native_field_ops, function.allow_witness} {
      %self = struct.new : <@Main<[]>>
      function.return %self : !struct.type<@Main<[]>>
    }
    function.def @constrain(%arg0: !struct.type<@Main<[]>>, %arg1: !struct.type<@Signal<[]>> {llzk.pub = #llzk.pub}) attributes {function.allow_constraint, function.allow_non_native_field_ops} {
      %0 = struct.readf %arg0[@adv_0_0] : <@Main<[]>>, !felt.type
      %1 = felt.neg %0 : !felt.type
      %2 = struct.readf %arg0[@adv_1_0] : <@Main<[]>>, !felt.type 
      constrain.eq %1, %2 : !felt.type, !felt.type
      %5 = felt.mul %0, %2 : !felt.type, !felt.type
      %6 = struct.readf %arg0[@adv_2_0] : <@Main<[]>>, !felt.type
      constrain.eq %5, %6 : !felt.type, !felt.type
      %8 = struct.readf %arg1[@reg] : <@Signal<[]>>, !felt.type
      constrain.eq %0, %8 : !felt.type, !felt.type
      %10 = struct.readf %arg0[@out_0] : <@Main<[]>>, !felt.type
      constrain.eq %6, %10 : !felt.type, !felt.type
      function.return
    }
    struct.field @adv_0_0 : !felt.type 
    struct.field @adv_1_0 : !felt.type 
    struct.field @adv_2_0 : !felt.type 
  }
}

