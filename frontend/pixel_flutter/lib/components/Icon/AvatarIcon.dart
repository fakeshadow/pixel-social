import 'package:flutter/material.dart';
import 'package:flutter_bloc/flutter_bloc.dart';

import 'package:pixel_flutter/blocs/UserBlocs.dart';
import 'package:pixel_flutter/style/colors.dart';

class AvatarIcon extends StatelessWidget {
  final String _url = 'http://192.168.1.197:3200';

  @override
  Widget build(BuildContext context) {
    final _userBloc = BlocProvider.of<UserBloc>(context);
    return Padding(
      padding: EdgeInsets.only(right:15),
      child: Material(
        elevation: 5,
        shape: CircleBorder(),
        child: Padding(
          padding: EdgeInsets.all(5.0),
          child: BlocBuilder(
              bloc: _userBloc,
              builder: (BuildContext context, UserState state) {
                if (state is UserLoaded) {
                  return CircleAvatar(
                      child: FadeInImage.assetNetwork(
                    placeholder: 'assets/test2.png',
                    image: '$_url${state.user.avatarUrl}',
                    fit: BoxFit.fitWidth,
                  ));
                }
                return Icon(Icons.apps,color: primaryColor,);
              }),
        ),
      ),
    );
  }
}
