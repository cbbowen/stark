const MouseEvent_offsetX_property_shim = Object.getOwnPropertyDescriptor(MouseEvent.prototype, 'offsetX').get;
export function MouseEvent_offset_x(mouse_event) {
  return MouseEvent_offsetX_property_shim.call(mouse_event);
}

const MouseEvent_offsetY_property_shim = Object.getOwnPropertyDescriptor(MouseEvent.prototype, 'offsetY').get;
export function MouseEvent_offset_y(mouse_event) {
  return MouseEvent_offsetY_property_shim.call(mouse_event);
}
