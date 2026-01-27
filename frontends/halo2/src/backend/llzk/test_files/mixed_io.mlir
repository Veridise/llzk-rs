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
    struct.field @out_1 : !felt.type {llzk.pub}
    struct.field @out_2 : !felt.type
    struct.field @out_3 : !felt.type
    function.def @compute(%arg0: !struct.type<@Signal<[]>> {llzk.pub = #llzk.pub}, %arg1: !struct.type<@Signal<[]>> {llzk.pub = #llzk.pub}, %arg2: !struct.type<@Signal<[]>>, %arg3: !struct.type<@Signal<[]>>, %arg4: !struct.type<@Signal<[]>>) -> !struct.type<@Main<[]>> attributes {function.allow_non_native_field_ops, function.allow_witness} {
      %self = struct.new : <@Main<[]>>
      function.return %self : !struct.type<@Main<[]>>
    }
    function.def @constrain(%arg0: !struct.type<@Main<[]>>, %arg1: !struct.type<@Signal<[]>> {llzk.pub = #llzk.pub}, %arg2: !struct.type<@Signal<[]>> {llzk.pub = #llzk.pub}, %arg3: !struct.type<@Signal<[]>>, %arg4: !struct.type<@Signal<[]>>, %arg5: !struct.type<@Signal<[]>>) attributes {function.allow_constraint, function.allow_non_native_field_ops} {
      function.return
    }
  }
}
