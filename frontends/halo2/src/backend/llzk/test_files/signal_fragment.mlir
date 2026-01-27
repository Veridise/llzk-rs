struct.def @Signal<[]> {
  struct.field @reg : !felt.type {llzk.pub}
  function.def @compute(%arg0: !felt.type) -> !struct.type<@Signal<[]>> attributes {function.allow_non_native_field_ops,function.allow_witness} {
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
