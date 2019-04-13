import 'package:flutter/material.dart';
import 'package:flutter_bloc/flutter_bloc.dart';

import 'package:pixel_flutter/blocs/UserBlocs.dart';

class AvatarIcon extends StatelessWidget {
  final String _url = 'http://192.168.1.197:3200';

  @override
  Widget build(BuildContext context) {
    final _userBloc = BlocProvider.of<UserBloc>(context);
    return BlocBuilder(
      bloc: _userBloc,
      builder: (BuildContext context, UserState state) {
        if (state is UserLoaded) {
          return IconButton(
            /// template logout
              onPressed: () => { _userBloc.dispatch(LoggingOut())},
              padding: const EdgeInsets.only(
                  top: 13, right: 9, left: 9, bottom: 13),
              icon: Stack(
                children: <Widget>[
                  Material(shape: CircleBorder(), child: CircleAvatar(
                      backgroundImage:
                      NetworkImage('$_url/public/' + state.user.avatarUrl))),
                  Positioned(
                      top: 0.0,
                      left: 0.0,
                      child: Icon(Icons.brightness_1,
                          size: 8.0, color: Colors.redAccent))
                ],
              ));
        } else {
          return IconButton(
              onPressed: () {},
              padding: const EdgeInsets.only(
                  top: 13, right: 9, left: 9, bottom: 13),
              icon: Stack(
                children: <Widget>[
                  Material(shape: CircleBorder(), child: CircleAvatar(
                      backgroundImage: AssetImage('assets/test2.png'))),
                  Positioned(
                      top: 0.0,
                      left: 0.0,
                      child: Icon(Icons.brightness_1,
                          size: 8.0, color: Colors.redAccent))
                ],
              ));
        }
      },
    );
  }
}


