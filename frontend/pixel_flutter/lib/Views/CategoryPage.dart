import 'package:flutter/material.dart';
import 'package:flutter_bloc/flutter_bloc.dart';
import 'package:pixel_flutter/Views/AutenticationPage.dart';

import 'package:pixel_flutter/blocs/HorizontalTabBlocs.dart';
import 'package:pixel_flutter/blocs/ErrorBlocs.dart';

import 'package:pixel_flutter/components/Background/GeneralBackground.dart';
import 'package:pixel_flutter/components/NavigationBar/HorizontalTab/HorizontalTabText.dart';
import 'package:pixel_flutter/components/Categories/CategoryHeader.dart';
import 'package:pixel_flutter/components/Categories/CategoryList.dart';
import 'package:pixel_flutter/components/Button/AddPostButton.dart';
import 'package:pixel_flutter/components/NavigationBar/CategoryNavBar.dart';

class CategoryPage extends StatefulWidget {
  @override
  _CategoryPageState createState() => _CategoryPageState();
}

class _CategoryPageState extends State<CategoryPage> {
  HorizontalTabBloc _tabBloc;
  ErrorBloc _errorBloc;

  @override
  void initState() {
    _tabBloc = HorizontalTabBloc();
    _errorBloc = BlocProvider.of<ErrorBloc>(context);
    super.initState();
  }

  @override
  void dispose() {
    _tabBloc.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
        body: BlocProvider(
            bloc: _tabBloc,
            child: BlocListener(
              bloc: _errorBloc,
              listener: (BuildContext context, ErrorState state) {
                if (state is NoSnack) {
                  Scaffold.of(context).hideCurrentSnackBar();
                }
                if (state is ShowSuccess) {
                  Scaffold.of(context).showSnackBar(SnackBar(
                    duration: Duration(seconds: 2),
                    backgroundColor: Colors.green,
                    content: Text(state.success),
                  ));
                }
                if (state is ShowError) {
                  Scaffold.of(context).showSnackBar(SnackBar(
                    duration: Duration(seconds: 2),
                    backgroundColor: Colors.deepOrangeAccent,
                    content: Text(state.error),
                  ));
                }
              },
              child: BlocBuilder(
                  bloc: _tabBloc,
                  builder: (BuildContext context, HorizontalTabState tabState) {
                    if (tabState is Selected) {
                      return Stack(
                        children: <Widget>[
                          GeneralBackground(),
                          Column(
                              crossAxisAlignment: CrossAxisAlignment.start,
                              children: <Widget>[
                                CatNavBar(),
                                CategoryHeader(
                                  tabIndex: tabState.index,
                                ),
                                Spacer(),
                                AddPostButton(text: 'New Topic')
                              ]),
                          Center(
                              child: Container(
                                  height: 470,
                                  child: CardStack(
                                    selectedTabIndex: tabState.index,
                                  )))
                        ],
                      );
                    }
                  }),
            )));
  }
}

class CardStack extends StatefulWidget {
  final int selectedTabIndex;

  CardStack({this.selectedTabIndex});

  @override
  _CardStackState createState() => _CardStackState();
}

class _CardStackState extends State<CardStack>
    with SingleTickerProviderStateMixin {
  AnimationController _animationController;
  Animation<double> _animationDouble;

  initAnimation() {
    _animationController.reset();
    _animationController.forward();
  }

  @override
  void initState() {
    _animationController =
        AnimationController(vsync: this, duration: Duration(milliseconds: 500));
    _animationDouble =
        Tween<double>(begin: 0.0, end: 1.0).animate(_animationController);
    super.initState();
  }

  @override
  void dispose() {
    _animationController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return Stack(
      children: <Widget>[
        Positioned(
          left: -20,
          top: 0,
          bottom: 0,
          width: 100,
          child: Padding(
            padding: EdgeInsets.symmetric(vertical: 80.0),
            child: Column(
              mainAxisAlignment: MainAxisAlignment.spaceBetween,
              children: <Widget>[
                HorizontalTabText(
                  text: 'Hot',
                  index: 0,
                ),
                HorizontalTabText(
                  text: 'Game',
                  index: 1,
                ),
                HorizontalTabText(
                  text: 'Talk',
                  index: 2,
                ),
              ],
            ),
          ),
        ),
        FutureBuilder(
          future: initAnimation(),
          builder: (context, snapshot) {
            return FadeTransition(
              opacity: _animationDouble,
              child: Padding(
                  padding: EdgeInsets.only(left: 60),
                  // ToDo: make separate class for list view on different tabs.
                  child: widget.selectedTabIndex == 0
                      ? CategoryList()
                      : widget.selectedTabIndex == 1
                          ? CategoryList()
                          : CategoryList()),
            );
          },
        ),
      ],
    );
  }
}
