import 'dart:math' as math;

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter_bloc/flutter_bloc.dart';
import 'package:pixel_flutter/Views/AutenticationPage.dart';

import 'package:pixel_flutter/blocs/UserBlocs.dart';
import 'package:pixel_flutter/style/colors.dart';

/// authentication page logic here
class AvatarIcon extends StatelessWidget {
  final String _url = 'http://192.168.1.197:3200';

  @override
  Widget build(BuildContext context) {
    final _userBloc = BlocProvider.of<UserBloc>(context);
    return BlocBuilder(
        bloc: _userBloc,
        builder: (BuildContext context, UserState state) {
          print('state is $state');
          return Hero(
            tag: State is UserLoaded ? 'profile' : 'auth',
            child: Material(
              child: SelfIconButton(
                color: primaryColor,
                padding: EdgeInsets.only(left: 0, top: 0, bottom: 0, right: 10),
                onPressed: () => state is UserLoaded
                    ? _userBloc.dispatch(LoggingOut())
                    : state is UserLoggedOut
                        ? Navigator.push(
                            context,
                            MaterialPageRoute(
                                builder: (context) => AuthenticationPage(
                                      type: 'Login', username: state.username,
                                    )))
                        : Navigator.push(
                            context,
                            MaterialPageRoute(
                                builder: (context) => AuthenticationPage(
                                      type: 'Register',
                                    ))),
                icon: state is UserLoaded
                    ? CircleAvatar(
                        backgroundImage:
                            NetworkImage('$_url${state.user.avatarUrl}'))
                    : Icon(Icons.apps),
                iconSize: state is UserLoaded ? 40 : 30,
              ),
            ),
          );
        });
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
    if (onPressed != null)
      currentColor = color;
    else
      currentColor = disabledColor ?? Theme.of(context).disabledColor;

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
        // x 0.5 for diameter -> radius and + 40% overflow derived from other Material apps.
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
