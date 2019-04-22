import 'package:flutter/material.dart';
import 'package:flutter_bloc/flutter_bloc.dart';
import 'package:pixel_flutter/blocs/ErrorBlocs.dart';
import 'package:pixel_flutter/blocs/TopicBlocs.dart';

import 'package:pixel_flutter/components/Background/GeneralBackground.dart';
import 'package:pixel_flutter/components/Menu/UserDrawer.dart';
import 'package:pixel_flutter/components/NavigationBar/SliverNavBar.dart';
import 'package:pixel_flutter/components/Loader/CenterLoader.dart';
import 'package:pixel_flutter/components/Loader/BottomLoader.dart';
import 'package:pixel_flutter/components/Topic/TopicList.dart';

import 'package:pixel_flutter/models/Category.dart';

class TopicsPage extends StatefulWidget {
  final Category category;

  TopicsPage({@required this.category});

  @override
  _TopicsPageState createState() => _TopicsPageState();
}

class _TopicsPageState extends State<TopicsPage> {
  int _categoryId;
  String _categoryName;
  String _categoryTheme;
  TopicBloc _topicBloc;
  ErrorBloc _errorBloc;
  final _scrollController = ScrollController();
  final _scrollThreshold = 200.0;

  @override
  void initState() {
    _categoryId = widget.category.id != null ? widget.category.id : 1;
    _categoryName =
        widget.category.name != null ? widget.category.name : 'PixelShare';
    _categoryTheme = widget.category.theme;
    _topicBloc = TopicBloc();
    _errorBloc = BlocProvider.of<ErrorBloc>(context);
    _errorBloc.dispatch(HideSnack());
    _topicBloc.dispatch(GetTopics(categoryId: _categoryId));
    _scrollController.addListener(_onScroll);
    super.initState();
  }

  @override
  Widget build(BuildContext context) {
    return BlocBuilder(
        bloc: _topicBloc,
        builder: (BuildContext context, TopicState state) {
          return WillPopScope(
            onWillPop: _hideSnack,
            child: Hero(
              tag: _categoryName,
              child: Scaffold(
                endDrawer: UserDrawer(),
                body: BlocListener(
                  bloc: _errorBloc,
                  listener: (BuildContext context, ErrorState state) {
                    if (state is NoSnack) {
                      Scaffold.of(context).hideCurrentSnackBar();
                    } else if (state is ShowError) {
                      Scaffold.of(context).showSnackBar(SnackBar(
                        duration: Duration(seconds: 2),
                        backgroundColor: Colors.deepOrangeAccent,
                        content: Text(state.error),
                      ));
                    } else if (state is ShowSuccess) {
                      Scaffold.of(context).showSnackBar(SnackBar(
                        duration: Duration(seconds: 2),
                        backgroundColor: Colors.green,
                        content: Text(state.success),
                      ));
                    }
                  },
                  child: Stack(
                    children: <Widget>[
                      GeneralBackground(),
                      CustomScrollView(
                          controller: _scrollController,
                          slivers: <Widget>[
                            SliverNavBar(
                                title: _categoryName, theme: _categoryTheme),
                            TopicList(state)
                          ])
                    ],
                  ),
                ),
              ),
            ),
          );
        });
  }

  @override
  void dispose() {
    _errorBloc.dispatch(HideSnack());
    _topicBloc.dispose();
    _scrollController.dispose();
    super.dispose();
  }

  Future<bool> _hideSnack() async {
   _errorBloc.dispatch(HideSnack());
   return true;
  }

  void _onScroll() {
    final maxScroll = _scrollController.position.maxScrollExtent;
    final currentScroll = _scrollController.position.pixels;
    if (maxScroll - currentScroll <= _scrollThreshold) {
      _topicBloc.dispatch(GetTopics(categoryId: _categoryId));
    }
  }
}

class TopicList extends StatelessWidget {
  final state;

  TopicList(this.state);

  @override
  Widget build(BuildContext context) {
    final _errorBloc = BlocProvider.of<ErrorBloc>(context);
    if (state is TopicError) {
      _errorBloc.dispatch(GetError(error: 'No topics found'));
      return CenterLoader();
    }
    if (state is TopicLoaded) {
      if (state.topics.isEmpty) {
        _errorBloc.dispatch(GetSuccess(success: 'No more topics'));
        return CenterLoader();
      }
      return SliverList(
        delegate: SliverChildBuilderDelegate((context, index) {
          return index >= state.topics.length
              ? BottomLoader()
              : TopicView(state.topics[index]);
        },
            childCount: state.hasReachedMax
                ? state.topics.length
                : state.topics.length + 1),
      );
    } else {
      return CenterLoader();
    }
  }
}
