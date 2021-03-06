import 'dart:math' as math;

import 'package:flutter_web/material.dart';

import 'package:pixel_flutter_web/env.dart';

import 'package:pixel_flutter_web/style/colors.dart';

class FloatingBarActionIcon extends StatelessWidget with env {
  final Function onPressed;
  final Widget icon;
  final double iconSize;

  FloatingBarActionIcon({this.onPressed, @required this.icon, @required this.iconSize});

  @override
  Widget build(BuildContext context) {
    return IconButton(
      color: primaryColor,
      padding: EdgeInsets.only(left: 0, top: 0, bottom: 0, right: 15),
      onPressed: onPressed != null ? () => onPressed() : () {},
      icon: icon,
      iconSize: iconSize,
    );
  }
}

class SelfIconButton extends StatelessWidget {
  const SelfIconButton(
      {Key key,
      this.iconSize = 24.0,
      this.padding = const EdgeInsets.all(0),
      this.alignment = Alignment.center,
      @required this.icon,
      this.color,
      this.highlightColor,
      this.splashColor,
      this.disabledColor,
      @required this.onPressed,
      this.tooltip})
      : assert(iconSize != null),
        assert(padding != null),
        assert(alignment != null),
        assert(icon != null),
        super(key: key);

  final double iconSize;
  final EdgeInsetsGeometry padding;
  final AlignmentGeometry alignment;
  final Widget icon;
  final Color color;
  final Color splashColor;
  final Color highlightColor;
  final Color disabledColor;
  final VoidCallback onPressed;
  final String tooltip;

  @override
  Widget build(BuildContext context) {
    assert(debugCheckHasMaterial(context));
    Color currentColor;
    if (onPressed != null) {
      currentColor = color;
    } else {
      currentColor = disabledColor ?? Theme.of(context).disabledColor;
    }
    Widget result = Semantics(
      button: true,
      enabled: onPressed != null,
      child: ConstrainedBox(
        constraints: const BoxConstraints(minWidth: 40, minHeight: 40),
        child: Padding(
          padding: padding,
          child: SizedBox(
            height: iconSize,
            width: iconSize,
            child: Align(
              alignment: alignment,
              child: IconTheme.merge(
                  data: IconThemeData(size: iconSize, color: currentColor),
                  child: icon),
            ),
          ),
        ),
      ),
    );

    if (tooltip != null) {
      result = Tooltip(message: tooltip, child: result);
    }
    return InkResponse(
      onTap: onPressed,
      child: result,
      highlightColor: highlightColor ?? Theme.of(context).highlightColor,
      splashColor: splashColor ?? Theme.of(context).splashColor,
      radius: math.max(
        Material.defaultSplashRadius,
        (iconSize + math.min(padding.horizontal, padding.vertical)) * 1,
      ),
    );
  }

  @override
  void debugFillProperties(DiagnosticPropertiesBuilder properties) {
    super.debugFillProperties(properties);
    properties.add(DiagnosticsProperty<Widget>('icon', icon, showName: false));
    properties.add(ObjectFlagProperty<VoidCallback>('onPressed', onPressed,
        ifNull: 'disabled'));
    properties.add(
        StringProperty('tooltip', tooltip, defaultValue: null, quoted: false));
  }
}
